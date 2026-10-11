//! Descriptive statistics calculated from decoded records without modifying them.
//! Index means are calculated ICF summaries, never additional measured values.
use crate::{
    work_records::WorkMeasurement,
    workspace::{Dataset, Work, WorkGroup},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq)]
pub(super) struct IndexSummary {
    /// Original integer tenths; presentation must convert explicitly to ICF.
    pub min: u16,
    pub max: u16,
    /// Calculated arithmetic mean in ICF units, without display rounding.
    pub mean: f64,
}

#[derive(Debug, PartialEq)]
pub(super) struct Summary {
    pub records: usize,
    pub groups: usize,
    pub points: usize,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    pub missing_dates: usize,
    pub missing_times: usize,
    pub missing_gps: usize,
    pub warned_records: usize,
    pub a: Option<IndexSummary>,
    pub b: Option<IndexSummary>,
    pub total: Option<IndexSummary>,
}

fn indices(
    records: &[&WorkMeasurement],
    value: impl Fn(&WorkMeasurement) -> u16,
) -> Option<IndexSummary> {
    let first = value(records.first()?);
    let mut min = first;
    let mut max = first;
    let mut sum = 0u64;
    for record in records {
        let value = value(record);
        min = min.min(value);
        max = max.max(value);
        sum += u64::from(value);
    }
    Some(IndexSummary {
        min,
        max,
        mean: sum as f64 / records.len() as f64 / 10.0,
    })
}

impl Summary {
    /// Records from one group (including a filtered selection). Point identifiers
    /// are unique only inside this group. Empty selections contain zero groups.
    pub fn records(records: &[&WorkMeasurement]) -> Self {
        let dates: Vec<_> = records
            .iter()
            .filter_map(|record| record.date.as_ref())
            .collect();
        Self {
            records: records.len(),
            groups: usize::from(!records.is_empty()),
            points: records
                .iter()
                .map(|record| record.point)
                .collect::<BTreeSet<_>>()
                .len(),
            first_date: dates.iter().min().map(|date| (*date).clone()),
            last_date: dates.iter().max().map(|date| (*date).clone()),
            missing_dates: records
                .iter()
                .filter(|record| record.date.is_none())
                .count(),
            missing_times: records
                .iter()
                .filter(|record| record.time.is_none())
                .count(),
            missing_gps: records
                .iter()
                .filter(|record| record.latitude.is_none() || record.longitude.is_none())
                .count(),
            warned_records: records
                .iter()
                .filter(|record| !record.warnings.is_empty())
                .count(),
            a: indices(records, |record| record.chlorophyll_a_tenths),
            b: indices(records, |record| record.chlorophyll_b_tenths),
            total: indices(records, |record| record.chlorophyll_total_tenths),
        }
    }

    pub fn group(group: &WorkGroup) -> Self {
        let mut summary = Self::records(&group.records.iter().collect::<Vec<_>>());
        summary.groups = 1;
        summary
    }

    pub fn work(work: &Work) -> Self {
        let mut summary = Self::records(
            &work
                .groups
                .iter()
                .flat_map(|group| &group.records)
                .collect::<Vec<_>>(),
        );
        summary.groups = work.groups.len();
        summary.points = work
            .groups
            .iter()
            .map(|group| Self::group(group).points)
            .sum();
        summary
    }

    pub fn dataset(dataset: &Dataset) -> Self {
        let mut summary = Self::records(
            &dataset
                .works
                .iter()
                .flat_map(|work| &work.groups)
                .flat_map(|group| &group.records)
                .collect::<Vec<_>>(),
        );
        summary.groups = dataset.works.iter().map(|work| work.groups.len()).sum();
        summary.points = dataset
            .works
            .iter()
            .map(|work| Self::work(work).points)
            .sum();
        summary
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct PointSummary {
    pub point: u16,
    pub summary: Summary,
}

/// Numerical point order. Repeated measurements remain separate observations.
pub(super) fn points(group: &WorkGroup) -> Vec<PointSummary> {
    let mut by_point: BTreeMap<u16, Vec<&WorkMeasurement>> = BTreeMap::new();
    for record in &group.records {
        by_point.entry(record.point).or_default().push(record);
    }
    by_point
        .into_iter()
        .map(|(point, records)| PointSummary {
            point,
            summary: Summary::records(&records),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_records::decode48;
    use crate::workspace::FilesystemAudit;

    fn record(point: u16, a: u16, b: u16, total: u16) -> WorkMeasurement {
        let mut raw = [0u8; 48];
        raw[..5].copy_from_slice(&[1, 2, 3, 4, 5]);
        for (offset, value) in [(24, a), (26, b), (28, total), (30, point), (32, 26)] {
            raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        raw[40..44].copy_from_slice(&(-12.25f32).to_le_bytes());
        raw[44..48].copy_from_slice(&34.5f32.to_le_bytes());
        decode48(
            &raw,
            vec![crate::records::Segment {
                file_offset: 32,
                memory_address: 0,
                length: 48,
            }],
        )
        .unwrap()
    }
    fn group(number: u16, records: Vec<WorkMeasurement>) -> WorkGroup {
        WorkGroup {
            source_file: format!("/trabalho1/{number}"),
            group: number,
            source_sha256: String::new(),
            expected_records: None,
            records,
        }
    }
    fn work(number: u16, groups: Vec<WorkGroup>) -> Work {
        Work {
            directory: format!("/trabalho{number}"),
            number,
            name: None,
            display_name: format!("trabalho{number}"),
            name_source: String::new(),
            configuration_file: None,
            expected_groups: None,
            groups,
            reference_validation: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    fn dataset(works: Vec<Work>) -> Dataset {
        Dataset {
            schema: String::new(),
            tool_version: String::new(),
            source_sha256: String::new(),
            source_size_bytes: 0,
            memory_sha256: String::new(),
            filesystem: FilesystemAudit {
                layout: String::new(),
                active_sector_count: 0,
                released_sector_count: 0,
                erased_sector_count: 0,
                crc_verified_count: 0,
                unreferenced_active_sectors: Vec::new(),
                directories: Vec::new(),
                sectors: Vec::new(),
                files: Vec::new(),
                limitations: Vec::new(),
            },
            works,
            unclassified_files: Vec::new(),
            validation: String::new(),
            limitations: Vec::new(),
        }
    }

    #[test]
    fn empty_records_have_no_numeric_statistics_or_nan() {
        let summary = Summary::records(&[]);
        assert_eq!((summary.records, summary.groups, summary.points), (0, 0, 0));
        assert!(summary.a.is_none() && summary.b.is_none() && summary.total.is_none());
        assert!(summary.first_date.is_none() && summary.last_date.is_none());
        assert_eq!(Summary::group(&group(1, Vec::new())).groups, 1);
        assert_eq!(Summary::work(&work(1, Vec::new())).groups, 0);
    }

    #[test]
    fn real_zero_and_independent_total_are_included_in_exact_means() {
        let first = record(1, 0, 0, 1);
        let second = record(1, 20, 40, 99);
        let before = serde_json::to_vec(&vec![&first, &second]).unwrap();
        let summary = Summary::records(&[&first, &second]);
        assert_eq!(
            summary.a,
            Some(IndexSummary {
                min: 0,
                max: 20,
                mean: 1.0
            })
        );
        assert_eq!(summary.b.as_ref().unwrap().mean, 2.0);
        assert_eq!(summary.total.as_ref().unwrap().mean, 5.0);
        assert_ne!(
            summary.total.as_ref().unwrap().mean,
            summary.a.as_ref().unwrap().mean + summary.b.as_ref().unwrap().mean
        );
        assert_eq!(summary.points, 1);
        assert_eq!(serde_json::to_vec(&vec![&first, &second]).unwrap(), before);
    }

    #[test]
    fn dates_times_partial_gps_and_warnings_are_counted_independently() {
        let mut first = record(1, 1, 2, 3);
        first.date = Some("2024-12-01".into());
        first.time = None;
        first.latitude = None;
        first.warnings = vec!["one".into(), "two".into()];
        let mut second = record(2, 4, 5, 6);
        second.date = None;
        second.longitude = None;
        let mut third = record(3, 7, 8, 9);
        third.date = Some("2024-03-12".into());
        let summary = Summary::records(&[&first, &second, &third]);
        assert_eq!(summary.first_date.as_deref(), Some("2024-03-12"));
        assert_eq!(summary.last_date.as_deref(), Some("2024-12-01"));
        assert_eq!(summary.missing_dates, 1);
        assert_eq!(summary.missing_times, 1);
        assert_eq!(summary.missing_gps, 2);
        assert_eq!(
            summary.warned_records, 1,
            "count records, not warning messages"
        );
    }

    #[test]
    fn work_and_dataset_keep_points_scoped_to_each_group() {
        let first = work(
            1,
            vec![
                group(1, vec![record(1, 10, 20, 40), record(1, 20, 30, 50)]),
                group(2, vec![record(1, 30, 40, 60)]),
            ],
        );
        let summary = Summary::work(&first);
        assert_eq!((summary.records, summary.groups, summary.points), (3, 2, 2));
        assert_eq!(summary.a.unwrap().mean, 2.0);
        let summary = Summary::dataset(&dataset(vec![
            first,
            work(2, vec![group(1, vec![record(1, 40, 50, 70)])]),
        ]));
        assert_eq!((summary.records, summary.groups, summary.points), (4, 3, 3));
        assert_eq!(summary.a.unwrap().mean, 2.5);
    }

    #[test]
    fn point_summaries_use_numeric_order_without_merging_observations() {
        let group = group(
            1,
            vec![
                record(10, 10, 20, 30),
                record(2, 20, 30, 40),
                record(1, 0, 0, 0),
                record(2, 40, 50, 70),
            ],
        );
        let points = points(&group);
        assert_eq!(
            points.iter().map(|point| point.point).collect::<Vec<_>>(),
            vec![1, 2, 10]
        );
        assert_eq!(points[1].summary.records, 2);
        assert_eq!(points[1].summary.a.as_ref().unwrap().mean, 3.0);
        assert_eq!(points[1].summary.total.as_ref().unwrap().mean, 5.5);
    }
}
