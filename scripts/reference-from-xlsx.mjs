// Read-only converter for the inspected Falker Leaf export layout.
// Node standard library only; bounded ZIP32 extraction, without subprocesses.
// Reject unsupported workbook layouts; never round or edit the original.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { inflateRawSync } from 'node:zlib';

const [xlsx, fc2, output, confirmation] = process.argv.slice(2);
if (!xlsx || !fc2 || !output || confirmation !== '--confirm-same-source') {
  throw new Error('Usage: node scripts/reference-from-xlsx.mjs reference.xlsx input.FC2 local/reference.json --confirm-same-source');
}
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const sourceBytes = fs.readFileSync(xlsx);
const inputHash = sha(fs.readFileSync(fc2));
function crc32(bytes) {
  let crc=0xffffffff;
  for (const b of bytes) {crc^=b; for(let bit=0;bit<8;bit++) crc=(crc>>>1)^((crc&1)?0xedb88320:0);}
  return (crc^0xffffffff)>>>0;
}
function unzipSelected(zip) {
  let eocd=-1;
  for(let p=zip.length-22;p>=Math.max(0,zip.length-65557);p--) {
    if(zip.readUInt32LE(p)===0x06054b50&&p+22+zip.readUInt16LE(p+20)===zip.length){eocd=p;break;}
  }
  if(eocd<0||zip.readUInt16LE(eocd+4)!==0||zip.readUInt16LE(eocd+6)!==0) throw new Error('Unsupported ZIP directory');
  const count=zip.readUInt16LE(eocd+10),start=zip.readUInt32LE(eocd+16),size=zip.readUInt32LE(eocd+12);
  if(count===65535||count>10000||start+size>eocd) throw new Error('Unsupported ZIP64 or oversized directory');
  const parts={};let p=start;
  for(let i=0;i<count;i++) {
    if(p+46>start+size||zip.readUInt32LE(p)!==0x02014b50) throw new Error('Malformed ZIP entry');
    const flags=zip.readUInt16LE(p+8),method=zip.readUInt16LE(p+10),crc=zip.readUInt32LE(p+16);
    const compressed=zip.readUInt32LE(p+20),length=zip.readUInt32LE(p+24),nameLength=zip.readUInt16LE(p+28);
    const extra=zip.readUInt16LE(p+30),comment=zip.readUInt16LE(p+32),local=zip.readUInt32LE(p+42);
    if(p+46+nameLength+extra+comment>start+size) throw new Error('Truncated ZIP directory entry');
    const name=zip.subarray(p+46,p+46+nameLength).toString('utf8');p+=46+nameLength+extra+comment;
    if(!['xl/sharedStrings.xml','xl/workbook.xml','xl/worksheets/sheet1.xml'].includes(name))continue;
    if(parts[name]!==undefined||flags&1||length>4*1024*1024||local+30>start||zip.readUInt32LE(local)!==0x04034b50) throw new Error('Unsupported ZIP entry');
    const data=local+30+zip.readUInt16LE(local+26)+zip.readUInt16LE(local+28);
    if(data+compressed>start) throw new Error('Truncated ZIP payload');
    const packed=zip.subarray(data,data+compressed);
    const plain=method===0?packed:method===8?inflateRawSync(packed,{maxOutputLength:4*1024*1024}):null;
    if(!plain||plain.length!==length||crc32(plain)!==crc)throw new Error('Unsupported compression or ZIP checksum mismatch');
    parts[name]=plain.toString('utf8');
  }
  return parts;
}
const parts = unzipSelected(sourceBytes);
if ((parts['xl/workbook.xml']?.match(/<sheet\b/g) ?? []).length !== 1) throw new Error('Expected a single-sheet export');
function unescapeXml(s) {
  return s.replace(/&(#x[0-9a-f]+|#\d+|quot|apos|lt|gt|amp);/gi, (_, entity) => {
    const named = {quot:'"', apos:"'", lt:'<', gt:'>', amp:'&'};
    return entity.startsWith('#') ? String.fromCodePoint(entity.startsWith('#x') ? parseInt(entity.slice(2),16) : Number(entity.slice(1))) : named[entity];
  });
}
const shared = [...(parts['xl/sharedStrings.xml'] ?? '').matchAll(/<si>([\s\S]*?)<\/si>/g)]
  .map(m => [...m[1].matchAll(/<t\b[^>]*>([\s\S]*?)<\/t>/g)].map(t => unescapeXml(t[1])).join(''));
const rows = new Map();
const sheet = parts['xl/worksheets/sheet1.xml'];
if (!sheet || /<f(?:\s|>)/.test(sheet)) throw new Error('Expected a plain-value export without formulas');
for (const row of sheet.matchAll(/<row\b[^>]*\br="(\d+)"[^>]*>([\s\S]*?)<\/row>/g)) {
  const cells = {};
  for (const cell of row[2].matchAll(/<c\b([^>]*)>([\s\S]*?)<\/c>/g)) {
    const address = cell[1].match(/\br="([A-Z]+)\d+"/)?.[1];
    const value = cell[2].match(/<v>([\s\S]*?)<\/v>/)?.[1];
    if (address && value !== undefined) cells[address] = /\bt="s"/.test(cell[1]) ? shared[Number(value)] : unescapeXml(value);
  }
  rows.set(Number(row[1]), cells);
}
const expected = ['Medição','Ponto','Hora','Data','Latitude','Longitude','Clorofila A','Clorofila B','Clorofila Total'];
const columns = ['B','C','D','E','F','G','H','I','J'];
if (!expected.every((s,i)=>rows.get(8)?.[columns[i]]===s)) throw new Error('Unexpected headers: this helper only supports the documented export layout');
const integer = (s,max) => { if (!/^\d+$/.test(s)) throw new Error(`Invalid integer ${s}`); const n=Number(s); if (!Number.isSafeInteger(n)||n>max) throw new Error('Integer out of bounds'); return n; };
function tenths(s) {
  const match=s?.match(/^(\d+)(?:\.(\d)0*)?$/);
  if (!match) throw new Error(`Index has unsupported precision ${s}; refusing rounding`);
  return integer(String(Number(match[1])*10+Number(match[2]??0)),65535);
}
const records=[];
for (const [number,cells] of rows) {
  if (number<=8 || !Object.keys(cells).length) continue;
  if (!columns.every(c=>cells[c]!==undefined)) throw new Error(`Incomplete data row ${number}`);
  const date=cells.E.match(/^(\d{2})-(\d{2})-(\d{4})$/);
  if (!date) throw new Error(`Unexpected date at row ${number}`);
  records.push({id:integer(cells.B,4294967295),point:integer(cells.C,65535),time:cells.D,
    date:`${date[3]}-${date[2]}-${date[1]}`,latitude:Number(cells.F),longitude:Number(cells.G),
    chlorophyll_a_tenths:tenths(cells.H),chlorophyll_b_tenths:tenths(cells.I),chlorophyll_total_tenths:tenths(cells.J)});
}
if (!records.length || records.some(r=>!Number.isFinite(r.latitude)||!Number.isFinite(r.longitude))) throw new Error('Missing records or invalid coordinates');
const manifest={schema:'fc2-reference-v1',source_fc2_sha256:inputHash,source_reference_sha256:sha(sourceBytes),
  source_reference_name:path.basename(xlsx),work:rows.get(6)?.B,confirmed_same_source:true,rows:records};
fs.writeFileSync(output,JSON.stringify(manifest,null,2)+'\n',{flag:'wx'});
console.log(`Extracted ${records.length} rows for ${manifest.work}; original workbook preserved.`);
