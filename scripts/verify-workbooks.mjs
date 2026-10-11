// Independent, local audit of generated OOXML, source offsets and reference order.
// Node standard library only. Does not alter original FC2/XLSX or generated files.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {inflateRawSync} from 'node:zlib';

const [directory='local/trabalhos',fc2='arquivo.FC2',reference='referencia.xlsx',output] = process.argv.slice(2);
const check=(condition,message)=>{if(!condition)throw new Error(message);};
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
function crc32(bytes){let crc=0xffffffff;for(const b of bytes){crc^=b;for(let bit=0;bit<8;bit++)crc=(crc>>>1)^((crc&1)?0xedb88320:0);}return(crc^0xffffffff)>>>0;}
function unzip(bytes){
  let end=-1;for(let i=bytes.length-22;i>=Math.max(0,bytes.length-65557);i--)if(bytes.readUInt32LE(i)===0x06054b50&&i+22+bytes.readUInt16LE(i+20)===bytes.length){end=i;break;}
  check(end>=0,'ZIP end directory missing');check(bytes.readUInt16LE(end+4)===0&&bytes.readUInt16LE(end+6)===0,'Multidisk ZIP unsupported');
  const count=bytes.readUInt16LE(end+10),start=bytes.readUInt32LE(end+16),size=bytes.readUInt32LE(end+12);check(start+size<=end,'ZIP directory bounds');
  check(count<65535,'ZIP64 unsupported');let p=start;const parts=new Map();
  for(let i=0;i<count;i++){
    check(p+46<=start+size&&bytes.readUInt32LE(p)===0x02014b50,'Bad ZIP entry');
    const flags=bytes.readUInt16LE(p+8),method=bytes.readUInt16LE(p+10),crc=bytes.readUInt32LE(p+16),packedSize=bytes.readUInt32LE(p+20),plainSize=bytes.readUInt32LE(p+24),nameLen=bytes.readUInt16LE(p+28),extra=bytes.readUInt16LE(p+30),comment=bytes.readUInt16LE(p+32),local=bytes.readUInt32LE(p+42);
    check(p+46+nameLen+extra+comment<=start+size,'ZIP filename bounds');const name=bytes.subarray(p+46,p+46+nameLen).toString('utf8');p+=46+nameLen+extra+comment;
    check(!parts.has(name)&&!(flags&1)&&plainSize<=64*1024*1024,'Duplicate, encrypted or oversized ZIP part');
    check(local+30<=start&&bytes.readUInt32LE(local)===0x04034b50,'ZIP local header');const a=local+30+bytes.readUInt16LE(local+26)+bytes.readUInt16LE(local+28);check(a+packedSize<=start,'ZIP payload bounds');
    const packed=bytes.subarray(a,a+packedSize);const plain=method===0?packed:method===8?inflateRawSync(packed,{maxOutputLength:64*1024*1024}):null;
    check(plain&&plain.length===plainSize&&crc32(plain)===crc,'ZIP size/CRC mismatch');parts.set(name,plain);
  }return parts;
}
function xmlText(s){return s.replace(/&(#x[0-9a-f]+|#\d+|quot|apos|lt|gt|amp);/gi,(_,entity)=>entity[0]==='#'?String.fromCodePoint(entity.startsWith('#x')?parseInt(entity.slice(2),16):Number(entity.slice(1))):({quot:'"',apos:"'",lt:'<',gt:'>',amp:'&'}[entity]));}
function parseSheet(xml,shared=[]){
  check(!/<f(?:\s|>)/.test(xml),'Unexpected formula in measurement sheet');const rows=new Map();
  for(const row of xml.matchAll(/<row\b[^>]*\br="(\d+)"[^>]*>([\s\S]*?)<\/row>/g)){
    const cells={};for(const c of row[2].matchAll(/<c\b([^>]*)>([\s\S]*?)<\/c>/g)){
      const column=c[1].match(/\br="([A-Z]+)\d+"/)?.[1];check(column,'Missing cell address');
      const raw=c[2].match(/<v>([\s\S]*?)<\/v>/)?.[1];const inline=[...c[2].matchAll(/<t\b[^>]*>([\s\S]*?)<\/t>/g)].map(m=>xmlText(m[1])).join('');
      cells[column]=/\bt="s"/.test(c[1])?shared[Number(raw)]:/\bt="inlineStr"/.test(c[1])?inline:raw===undefined?null:xmlText(raw);
    }rows.set(Number(row[1]),cells);
  }return rows;
}
const original=fs.readFileSync(fc2);const dataset=JSON.parse(fs.readFileSync(path.join(directory,'dataset.json')));const manifest=JSON.parse(fs.readFileSync(path.join(directory,'manifest.json')));
check(sha(original)===dataset.source_sha256&&manifest.source_sha256===dataset.source_sha256,'FC2 provenance hash mismatch');
const expectedHeaders=['Medição','Ponto','Hora','Data','Latitude','Longitude','Clorofila A','Clorofila B','Clorofila Total'];const columns=['B','C','D','E','F','G','H','I','J'];
let recordCount=0,sheetCount=0,sourceSegments=0,nullDates=0,nullTimes=0,nullLatitudes=0,nullLongitudes=0;const workbookReports=[];let rows135;
const bundle=unzip(fs.readFileSync(path.join(directory,'todos-trabalhos.zip')));
for(const work of dataset.works){
  const item=manifest.works.find(i=>i.number===work.number);check(item&&item.filename&&!item.error,`Missing workbook for work ${work.number}`);
  const bytes=fs.readFileSync(path.join(directory,item.filename));check(bundle.get(item.filename)?.equals(bytes),`ZIP bundle differs for ${item.filename}`);const parts=unzip(bytes);
  const workbook=parts.get('xl/workbook.xml').toString();const sheets=[...workbook.matchAll(/<sheet\b([^>]*)\/>/g)];check(sheets.length===work.groups.length,`Sheet count ${work.number}`);
  check(parts.get('docProps/custom.xml').toString().includes(dataset.source_sha256),'Missing custom-property source hash');const sheetReports=[];
  for(let groupIndex=0;groupIndex<work.groups.length;groupIndex++){
    const group=work.groups[groupIndex],name=xmlText(sheets[groupIndex][1].match(/\bname="([^"]+)"/)[1]);check(name===(work.groups.length===1?'Worksheet':`Medidas ${group.group}`),`Sheet group identity ${work.number}`);
    const rows=parseSheet(parts.get(`xl/worksheets/sheet${groupIndex+1}.xml`).toString());check(expectedHeaders.every((h,i)=>rows.get(8)?.[columns[i]]===h),'Header mismatch');
    const measurements=[...rows].filter(([n])=>n>=9).map(([,cells])=>cells);const records=[...group.records];if(manifest.order==='lexicographic_point_then_active_file_record_order')records.sort((a,b)=>{const x=a.point.toString(),y=b.point.toString();return x<y?-1:x>y?1:0;});
    check(records.length===measurements.length,`Record count ${work.number}/${group.group}`);
    if(work.number===135&&groupIndex===0)rows135=measurements;
    for(let i=0;i<records.length;i++){
      const r=records[i],c=measurements[i],label=`${work.number}/${group.group}/${r.id}`;
      const raw=Buffer.concat(r.segments.map(s=>{check(Number.isSafeInteger(s.file_offset)&&Number.isSafeInteger(s.length)&&s.file_offset>=0&&s.length>0&&s.file_offset+s.length<=original.length,`Source bounds ${label}`);sourceSegments++;return original.subarray(s.file_offset,s.file_offset+s.length);}));
      check(raw.length===48&&raw.toString('hex')===r.raw_hex&&sha(raw)===r.raw_sha256,`Raw provenance ${label}`);
      const integerFields={id:[36,'readUInt32LE'],point:[30,'readUInt16LE'],chlorophyll_a_tenths:[24,'readUInt16LE'],chlorophyll_b_tenths:[26,'readUInt16LE'],chlorophyll_total_tenths:[28,'readUInt16LE']};
      for(const [field,[offset,method]]of Object.entries(integerFields))check(raw[method](offset)===r[field],`Raw integer ${label}/${field}`);
      check(raw.readUInt32LE(40)===r.latitude_bits&&raw.readUInt32LE(44)===r.longitude_bits,`Coordinate bits ${label}`);
      for(const [field,offset,bound]of [['latitude',40,90],['longitude',44,180]]){const value=raw.readFloatLE(offset);check(r[field]===(Number.isFinite(value)&&Math.abs(value)<=bound?value:null),`Decoded coordinate ${label}/${field}`);}
      const calendar={second:raw[0],minute:raw[1],hour:raw[2],day:raw[3],month:raw[4],year:raw.readUInt16LE(32)};
      check(Object.entries(calendar).every(([k,v])=>r.raw_calendar[k]===v),`Calendar bytes ${label}`);
      const empty=Object.values(calendar).every(v=>v===0);let date=null,time=null;
      if(!empty){const y=2000+calendar.year,actual=new Date(Date.UTC(y,calendar.month-1,calendar.day));if(calendar.year<=99&&calendar.month>=1&&calendar.month<=12&&calendar.day>0&&actual.getUTCFullYear()===y&&actual.getUTCMonth()===calendar.month-1&&actual.getUTCDate()===calendar.day)date=`${y}-${String(calendar.month).padStart(2,'0')}-${String(calendar.day).padStart(2,'0')}`;if(calendar.hour<24&&calendar.minute<60&&calendar.second<60)time=[calendar.hour,calendar.minute,calendar.second].map(v=>String(v).padStart(2,'0')).join(':');}
      check(r.date===date&&r.time===time,`Calendar interpretation ${label}`);
      const numeric={B:r.id,C:r.point,F:r.latitude,G:r.longitude,H:r.chlorophyll_a_tenths/10,I:r.chlorophyll_b_tenths/10,J:r.chlorophyll_total_tenths/10};
      for(const [column,value]of Object.entries(numeric))check(value===null?c[column]===undefined:c[column]!==undefined&&Number(c[column])===value,`XLSX number ${label}/${column}`);
      check(r.time===null?c.D===undefined:c.D===r.time,`XLSX time ${label}`);check(r.date===null?c.E===undefined:c.E===r.date.split('-').reverse().join('-'),`XLSX date ${label}`);
      if(r.date===null)nullDates++;if(r.time===null)nullTimes++;if(r.latitude===null)nullLatitudes++;if(r.longitude===null)nullLongitudes++;recordCount++;
    }
    sheetReports.push({name,group:group.group,records:records.length});sheetCount++;
  }workbookReports.push({work:work.number,filename:item.filename,sha256:sha(bytes),sheets:sheetReports});
}
check(recordCount===manifest.record_count&&sheetCount===manifest.group_count,'Manifest totals mismatch');
const referenceParts=unzip(fs.readFileSync(reference));const shared=[...(referenceParts.get('xl/sharedStrings.xml')?.toString()??'').matchAll(/<si>([\s\S]*?)<\/si>/g)].map(m=>[...m[1].matchAll(/<t\b[^>]*>([\s\S]*?)<\/t>/g)].map(v=>xmlText(v[1])).join(''));
const referenceSheet=parseSheet(referenceParts.get('xl/worksheets/sheet1.xml').toString(),shared);check(typeof referenceSheet.get(6)?.B==='string','Reference work name is missing');check(expectedHeaders.every((h,i)=>referenceSheet.get(8)?.[columns[i]]===h),'Reference header mismatch');
const referenceRows=[...referenceSheet].filter(([n])=>n>=9).map(([,r])=>r);check(rows135&&rows135.length===referenceRows.length,'Reference work count mismatch');let maxLatitudeDifference=0,maxLongitudeDifference=0;
for(let i=0;i<rows135.length;i++){const a=rows135[i],b=referenceRows[i];for(const col of ['B','C','H','I','J'])check(Number(a[col])===Number(b[col]),`Original reference field/order row ${i}/${col}`);check(a.D===b.D&&a.E===b.E,`Reference time/date row ${i}`);const lat=Math.abs(Number(a.F)-Number(b.F)),lon=Math.abs(Number(a.G)-Number(b.G));check(lat<=5e-7&&lon<=5e-7,`Reference coordinate precision row ${i}`);maxLatitudeDifference=Math.max(maxLatitudeDifference,lat);maxLongitudeDifference=Math.max(maxLongitudeDifference,lon);}
const report={accepted:true,source_sha256:dataset.source_sha256,workbooks:workbookReports.length,sheets:sheetCount,records:recordCount,source_segments_checked:sourceSegments,numeric_values_exact_to_decoded_raw:true,raw_hashes_and_offsets_verified:true,missing_cells_verified:{date:nullDates,time:nullTimes,latitude:nullLatitudes,longitude:nullLongitudes},bundle_workbook_bytes_identical:true,reference135:{records:rows135.length,identifiers_points_indices_time_date_and_order_exact:true,coordinate_tolerance:5e-7,max_latitude_difference:maxLatitudeDifference,max_longitude_difference:maxLongitudeDifference},workbook_details:workbookReports};
const reportBytes=JSON.stringify(report,null,2)+'\n';if(output)fs.writeFileSync(output,reportBytes,{flag:'wx'});console.log(JSON.stringify({...report,workbook_details:undefined},null,2));
