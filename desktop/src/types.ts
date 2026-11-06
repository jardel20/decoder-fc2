export interface WorkSummary {
  key: number;
  number: number;
  name: string;
  records: number;
  groups: number;
  points: number;
  coordinates: number;
  warnings: number;
  errors: string[];
  directory: string;
  notes: string[];
  first_date: string | null;
  last_date: string | null;
}
export interface SourceSummary {
  name: string;
  path: string;
  crc_verified: number;
  sha256: string;
  size: number;
  records: number;
  groups: number;
  coordinates: number;
  limitations: string[];
  works: WorkSummary[];
}
export interface Measurement {
  id: number;
  point: number;
  date: string | null;
  time: string | null;
  latitude: number | null;
  longitude: number | null;
  chlorophyll_a_tenths: number;
  chlorophyll_b_tenths: number;
  chlorophyll_total_tenths: number;
  chlorophyll_a_index: string;
  chlorophyll_b_index: string;
  chlorophyll_total_index: string;
  raw_hex: string;
  raw_sha256: string;
  warnings: string[];
  segments: { file_offset: number; memory_address: number; length: number }[];
}
export interface WorkDetail {
  number: number;
  display_name: string;
  directory: string;
  name_source: string;
  configuration_file: string | null;
  warnings: string[];
  errors: string[];
  groups: { group: number; source_file: string; records: Measurement[] }[];
}
export interface ExportResult {
  path: string;
  workbooks: number;
  failures: number;
  records: number;
  errors: string[];
}
export interface Location {
  work: number;
  work_number: number;
  work_name: string;
  group_index: number;
  group: number;
  record: number;
  id: number;
  point: number;
  date: string | null;
  time: string | null;
  a: string;
  b: string;
  total: string;
  latitude: number;
  longitude: number;
  warnings: string[];
}
