use super::Export;
use crate::error::Result;

fn quote(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

pub fn encode(document: &Export) -> Result<Vec<u8>> {
    super::ensure_validated(document)?;
    let mut output = String::from(
        "source_name,source_sha256,reference_sha256,work,id,point,date,time,timezone,latitude,longitude,latitude_bits,longitude_bits,chlorophyll_a_index,chlorophyll_b_index,chlorophyll_total_index,chlorophyll_a_tenths,chlorophyll_b_tenths,chlorophyll_total_tenths,reference_row_index,order,validation,occurrence_count,raw_hex,raw_sha256,occurrences_json\n",
    );
    for record in &document.records {
        let raw = record
            .occurrences
            .first()
            .map(|o| o.raw_hex.clone())
            .unwrap_or_default();
        let cells = vec![
            document.source_name.clone(),
            document.source_sha256.clone(),
            document.reference_sha256.clone(),
            record.work.clone(),
            record.id.to_string(),
            record.point.to_string(),
            record.date.clone(),
            record.time.clone(),
            record.timezone.clone().unwrap_or_default(),
            record.latitude.to_string(),
            record.longitude.to_string(),
            record.latitude_bits.to_string(),
            record.longitude_bits.to_string(),
            record.chlorophyll_a_index.clone(),
            record.chlorophyll_b_index.clone(),
            record.chlorophyll_total_index.clone(),
            record.chlorophyll_a_tenths.to_string(),
            record.chlorophyll_b_tenths.to_string(),
            record.chlorophyll_total_tenths.to_string(),
            record.reference_row_index.to_string(),
            document.order.clone(),
            record.validation.clone(),
            record.occurrences.len().to_string(),
            raw,
            record
                .occurrences
                .first()
                .map(|o| o.raw_sha256.clone())
                .unwrap_or_default(),
            serde_json::to_string(&record.occurrences)?,
        ];
        output.push_str(&cells.iter().map(|s| quote(s)).collect::<Vec<_>>().join(","));
        output.push('\n');
    }
    Ok(output.into_bytes())
}
