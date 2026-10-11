// Independent, read-only reconciliation of Rust CSV/JSON exports and raw offsets.
import fs from 'node:fs';
import crypto from 'node:crypto';
const [input,referencePath,jsonPath,csvPath]=process.argv.slice(2);
if(!csvPath)throw new Error('Usage: node scripts/verify-local-export.mjs input.FC2 reference.json export.json export.csv');
const raw=fs.readFileSync(input),reference=JSON.parse(fs.readFileSync(referencePath)),doc=JSON.parse(fs.readFileSync(jsonPath));
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const check=(condition,message)=>{if(!condition)throw new Error(message);};
const decimal=n=>`${Math.floor(n/10)}.${n%10}`;
function parseCSV(text) {
  const rows=[];let row=[],cell='',quoted=false;
  for(let i=0;i<text.length;i++) {
    const c=text[i];
    if(quoted) {if(c==='"'){if(text[i+1]==='"'){cell+='"';i++;}else quoted=false;}else cell+=c;}
    else if(c==='"'){check(cell==='','unexpected quote');quoted=true;}
    else if(c===','){row.push(cell);cell='';}
    else if(c==='\n'){row.push(cell);rows.push(row);row=[];cell='';}
    else if(c!=='\r')cell+=c;
  }
  check(!quoted,'unclosed quote');if(row.length||cell){row.push(cell);rows.push(row);}return rows;
}
const [headers,...csvRows]=parseCSV(fs.readFileSync(csvPath,'utf8'));
check(doc.records.length===reference.rows.length && csvRows.length===reference.rows.length,'row count mismatch');
check(doc.source_sha256===sha(raw)&&reference.source_fc2_sha256===sha(raw),'source hash mismatch');
const expected=new Map(reference.rows.map(r=>[r.id,r]));let maxLatitude=0,maxLongitude=0,copies=0,crossSector=0;
for(let i=0;i<doc.records.length;i++) {
  const r=doc.records[i],ref=expected.get(r.id);check(ref!==undefined,'unexpected identifier');expected.delete(r.id);
  const csv=Object.fromEntries(headers.map((key,n)=>[key,csvRows[i][n]]));check(csvRows[i].length===headers.length,'CSV column count mismatch');
  for(const key of ['id','point','date','time','chlorophyll_a_tenths','chlorophyll_b_tenths','chlorophyll_total_tenths']) {
    check(r[key]===ref[key],`reference mismatch ${r.id} ${key}`);check(csv[key]===String(r[key]),`CSV mismatch ${key}`);
  }
  for(const key of ['chlorophyll_a','chlorophyll_b','chlorophyll_total']) {
    check(r[`${key}_index`]===decimal(ref[`${key}_tenths`]),`index mismatch ${key}`);
    check(csv[`${key}_index`]===r[`${key}_index`],`CSV decimal mismatch ${key}`);
  }
  maxLatitude=Math.max(maxLatitude,Math.abs(r.latitude-ref.latitude));maxLongitude=Math.max(maxLongitude,Math.abs(r.longitude-ref.longitude));
  check(Math.abs(r.latitude-ref.latitude)<=5e-7&&Math.abs(r.longitude-ref.longitude)<=5e-7,'coordinate mismatch');
  check(Number(csv.latitude)===r.latitude&&Number(csv.longitude)===r.longitude,'CSV coordinates changed');
  check(csv.source_sha256===doc.source_sha256&&csv.reference_sha256===doc.reference_sha256,'CSV provenance mismatch');
  check(JSON.stringify(JSON.parse(csv.occurrences_json))===JSON.stringify(r.occurrences),'CSV raw provenance changed');
  const crosses=r.occurrences.some(o=>o.segments.some((s,n)=>n>0&&s.memory_address!==o.segments[n-1].memory_address+o.segments[n-1].length));
  if(crosses)crossSector++;
  for(const o of r.occurrences) {
    const bytes=Buffer.concat(o.segments.map(s=>{check(s.file_offset>=0&&s.file_offset+s.length<=raw.length,'raw bounds');return raw.subarray(s.file_offset,s.file_offset+s.length);}));
    check(bytes.length===48&&bytes.toString('hex')===o.raw_hex&&sha(bytes)===o.raw_sha256,'raw bytes or hash mismatch');
    check(bytes.readUInt32LE(36)===r.id&&bytes.readUInt16LE(30)===r.point,'raw identifier mismatch');
    check(bytes.readUInt16LE(24)===r.chlorophyll_a_tenths&&bytes.readUInt16LE(26)===r.chlorophyll_b_tenths&&bytes.readUInt16LE(28)===r.chlorophyll_total_tenths,'raw index mismatch');
    check(bytes.readFloatLE(40)===r.latitude&&bytes.readFloatLE(44)===r.longitude,'float payload changed');
    check(bytes.readUInt32LE(40)===r.latitude_bits&&bytes.readUInt32LE(44)===r.longitude_bits,'float bit pattern changed');copies++;
  }
}
check(expected.size===0,'missing identifiers');
console.log(JSON.stringify({accepted:true,records:doc.records.length,csv_records:csvRows.length,physical_reconstructions:copies,
  records_crossing_linked_sectors:crossSector,max_latitude_difference:maxLatitude,max_longitude_difference:maxLongitude,
  source_sha256:sha(raw),json_sha256:sha(fs.readFileSync(jsonPath)),csv_sha256:sha(fs.readFileSync(csvPath))},null,2));
