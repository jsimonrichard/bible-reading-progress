use std::ops::Range;
use std::{collections::BTreeMap, ops::RangeInclusive};

use serde::{Deserialize, Serialize};

pub trait CanCoalesce {
    fn coalesce(&self, other: &Self) -> Option<Self>
    where
        Self: Sized;
}

impl<T: Eq + Clone> CanCoalesce for T {
    fn coalesce(&self, other: &Self) -> Option<Self> {
        if self == other {
            Some(self.clone())
        } else {
            None
        }
    }
}

/// A map of disjoint half-open ranges `Range<T>` and values V where
/// an overlap (during insert) triggers merging of the values on the
/// intersection of the overlapping ranges
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RangeMap<K, V>
where
    K: Ord + Copy,
    V: Clone,
{
    // Invariant: keys are disjoint and in order.
    map: BTreeMap<K, (K, V)>, // start -> (end, value)
}

impl<K, V> RangeMap<K, V>
where
    K: Ord + Copy,
    V: Clone + CanCoalesce,
{
    /// Create an empty set.
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn range(&self, range: Range<K>) -> impl Iterator<Item = (Range<&K>, &V)> + '_ {
        let left = self
            .map
            .range(..range.start)
            .next_back()
            .filter(|(_, (e, _))| *e > range.start);
        let right = self.map.range(range);
        let iter = left.into_iter().chain(right.into_iter());
        return iter.map(|(start, (end, value))| (start..end, value));
    }

    fn range_biinclusive(
        &self,
        range: RangeInclusive<K>,
    ) -> impl Iterator<Item = (Range<&K>, &V)> + '_ {
        let left = self
            .map
            .range(..range.start())
            .next_back()
            .filter(|(_, (e, _))| e >= range.start());
        let right = self.map.range(range);
        let iter = left.into_iter().chain(right.into_iter());
        return iter.map(|(start, (end, value))| (start..end, value));
    }

    /// Insert a range with a value, splitting/merging as necessary.
    /// `merge(old, new)` is called for every overlap to produce the
    /// value that will be stored in the overlapping piece.
    pub fn insert_with<F>(&mut self, range: Range<K>, value: V, merge: F)
    where
        F: Fn(&V, &V) -> V,
    {
        let mut to_insert = Vec::new();
        let mut to_remove = Vec::new();

        let mut start_cursor = range.start;

        if let Some((s, (e, v))) = self.map.range_mut(..range.start).next_back() {
            if start_cursor == *e {
                if let Some(v2) = v.coalesce(&value) {
                    *v = v2;
                    start_cursor = *s;
                }
            } else if start_cursor < *e {
                if let Some(v2) = v.coalesce(&value) {
                    *v = v2;
                    start_cursor = *s;
                } else {
                    if range.end < *e {
                        to_insert.push((range.end, (*e, v.clone())));
                    }

                    let next_end = (*e).min(range.end);
                    *e = start_cursor;
                    to_insert.push((start_cursor, (next_end, merge(v, &value))));
                    start_cursor = next_end;
                }
            }
        }

        let mut coalesced_value = value.clone();

        for (s, (e, v)) in self.map.range_mut(range.start..range.end) {
            if let Some(v2) = v.coalesce(&coalesced_value) {
                coalesced_value = v2;
                to_remove.push(s.clone());
                continue;
            }

            if start_cursor < *s {
                // no overlap here
                to_insert.push((start_cursor, (s.clone(), coalesced_value)));
                coalesced_value = value.clone();
                // no need to update start_cursor here
            }
            // conceptually, start_cursor == *s
            if *e <= range.end {
                *v = merge(v, &coalesced_value);
                start_cursor = *e;
            } else {
                to_insert.push((range.end, (*e, v.clone())));
                *e = range.end;
                *v = merge(v, &coalesced_value);
                start_cursor = range.end;
                // this is the last loop
            }
        }

        if start_cursor < range.end {
            to_insert.push((start_cursor, (range.end, coalesced_value)));
        }

        for s in to_remove {
            self.map.remove(&s);
        }

        for (s, v) in to_insert {
            self.map.insert(s, v);
        }

        // Coalesce
        self.coalesce_in_range(range.start..=range.end);
    }

    pub fn coalesce_in_range(&mut self, range: RangeInclusive<K>) {
        let mut cursor = None;
        let mut to_remove = Vec::new();
        let mut to_update = Vec::new();
        for (r, v) in self.range_biinclusive(range) {
            let Some((start, end, curr_value, count)) = cursor else {
                cursor = Some((r.start, r.end, v.clone(), 1));
                continue;
            };

            if r.start == end {
                if let Some(v2) = v.coalesce(&curr_value) {
                    to_remove.push(r.start.clone());
                    cursor = Some((start, r.end, v2, count + 1));
                    continue;
                }
            }

            if count > 1 {
                to_update.push((start.clone(), end.clone(), curr_value));
            }

            // Start a new range
            cursor = Some((r.start, r.end, v.clone(), 1));
        }

        if let Some((start, end, value, count)) = cursor {
            if count > 1 {
                to_update.push((start.clone(), end.clone(), value));
            }
        }

        for s in to_remove {
            self.map.remove(&s);
        }

        for (s, e, value) in to_update {
            if let Some((old_e, old_value)) = self.map.get_mut(&s) {
                *old_e = e;
                *old_value = value;
            } else {
                unreachable!()
            }
        }
    }

    /// Simple insert that keeps the new value on overlap.
    pub fn insert_replace(&mut self, range: Range<K>, value: V) {
        self.insert_with(range, value, |_, new| new.clone());
    }

    /// Iterator over disjoint ranges and their values.
    pub fn iter(&self) -> impl Iterator<Item = (Range<K>, &V)> + '_ {
        self.map.iter().map(|(&s, &(e, ref v))| (s..e, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_merge() {
        let mut rm = RangeMap::new();
        rm.insert_replace(2..4, "B");

        // merge function: choose the new value
        rm.insert_with(1..3, "A", |_, new| new);

        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(1..3, "A"), (3..4, "B")]);
    }

    #[test]
    fn empty_map() {
        let rm = RangeMap::<i32, &str>::new();
        let v: Vec<_> = rm.iter().collect();
        assert_eq!(v, vec![]);
    }

    #[test]
    fn single_insertion() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, "A");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..10, "A")]);
    }

    #[test]
    fn non_overlapping_insertions() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(10..15, "B");
        rm.insert_replace(20..25, "C");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (10..15, "B"), (20..25, "C")]);
    }

    #[test]
    fn adjacent_ranges() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(5..10, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (5..10, "B")]);
    }

    #[test]
    fn overlapping_replace() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, "A");
        rm.insert_replace(5..15, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (5..15, "B")]);
    }

    #[test]
    fn complete_overlap_replace() {
        let mut rm = RangeMap::new();
        rm.insert_replace(5..10, "A");
        rm.insert_replace(0..20, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..20, "B")]);
    }

    #[test]
    fn multiple_overlaps_replace() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(10..15, "B");
        rm.insert_replace(20..25, "C");
        rm.insert_replace(3..22, "D");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..3, "A"), (3..22, "D"), (22..25, "C")]);
    }

    #[test]
    fn merge_same_values() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(5..10, "A"); // Same value, should merge
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..10, "A")]);
    }

    #[test]
    fn merge_same_values_overlapping() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(3..8, "A"); // Same value, overlapping, should merge
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..8, "A")]);
    }

    #[test]
    fn merge_with_function() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, 10);
        rm.insert_with(5..15, 20, |old, new| old + new);
        let v: Vec<_> = rm.iter().map(|(r, &n)| (r, n)).collect();
        assert_eq!(v, vec![(0..5, 10), (5..10, 30), (10..15, 20)]);
    }

    #[test]
    fn merge_keep_old() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, "A");
        rm.insert_with(5..15, "B", |old, _| *old);
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..10, "A"), (10..15, "B")]);
    }

    #[test]
    fn range_query_no_overlap() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(10..15, "B");
        rm.insert_replace(20..25, "C");
        // Query 5..10: returns last range starting < 5, and ranges starting >= 10
        let v: Vec<_> = rm
            .range(4..11)
            .map(|(r, &s)| (*r.start..*r.end, s))
            .collect();
        assert_eq!(v, vec![(0..5, "A"), (10..15, "B")]);
    }

    #[test]
    fn range_query_single_overlap() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(10..15, "B");
        rm.insert_replace(20..25, "C");
        // Query 2..7: returns last range starting < 2, and ranges starting >= 7
        let v: Vec<_> = rm
            .range(2..7)
            .map(|(r, &s)| (*r.start..*r.end, s))
            .collect();
        assert_eq!(v, vec![(0..5, "A")]);
    }

    #[test]
    fn range_query_multiple_overlaps() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(10..15, "B");
        rm.insert_replace(20..25, "C");
        // Query 3..22: returns last range starting < 3, and ranges starting >= 22
        let v: Vec<_> = rm
            .range(3..22)
            .map(|(r, &s)| (*r.start..*r.end, s))
            .collect();
        assert_eq!(v, vec![(0..5, "A"), (10..15, "B"), (20..25, "C")]);
    }

    #[test]
    fn insert_before_existing() {
        let mut rm = RangeMap::new();
        rm.insert_replace(10..20, "B");
        rm.insert_replace(0..5, "A");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (10..20, "B")]);
    }

    #[test]
    fn insert_after_existing() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(10..20, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (10..20, "B")]);
    }

    #[test]
    fn insert_overlapping_start() {
        let mut rm = RangeMap::new();
        rm.insert_replace(5..15, "B");
        rm.insert_replace(0..10, "A");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..10, "A"), (10..15, "B")]);
    }

    #[test]
    fn insert_overlapping_end() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, "A");
        rm.insert_replace(5..15, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (5..15, "B")]);
    }

    #[test]
    fn insert_contained_within_existing() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..20, "A");
        rm.insert_replace(5..15, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (5..15, "B"), (15..20, "A")]);
    }

    #[test]
    fn insert_containing_existing() {
        let mut rm = RangeMap::new();
        rm.insert_replace(5..15, "A");
        rm.insert_replace(0..20, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..20, "B")]);
    }

    #[test]
    fn complex_merge_scenario() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, 1);
        rm.insert_replace(10..15, 2);
        rm.insert_replace(20..25, 3);
        // Insert range that overlaps all three
        rm.insert_with(3..23, 10, |old, new| old + new);
        let v: Vec<_> = rm.iter().map(|(r, &n)| (r, n)).collect();
        assert_eq!(
            v,
            vec![
                (0..3, 1),
                (3..5, 11),
                (5..10, 10),
                (10..15, 12),
                (15..20, 10),
                (20..23, 13),
                (23..25, 3)
            ]
        );
    }

    #[test]
    fn single_point_ranges() {
        let mut rm = RangeMap::new();
        rm.insert_replace(5..6, "A");
        rm.insert_replace(10..11, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(5..6, "A"), (10..11, "B")]);
    }

    #[test]
    fn overlapping_single_point() {
        let mut rm = RangeMap::new();
        rm.insert_replace(5..6, "A");
        rm.insert_replace(5..6, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(5..6, "B")]);
    }

    #[test]
    fn range_query_at_boundaries() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..5, "A");
        rm.insert_replace(5..10, "B");
        // Query exactly at boundary
        let v: Vec<_> = rm
            .range(5..5)
            .map(|(r, &s)| (*r.start..*r.end, s))
            .collect();
        assert_eq!(v, vec![]);
        // Query touching boundary
        let v: Vec<_> = rm
            .range(4..6)
            .map(|(r, &s)| (*r.start..*r.end, s))
            .collect();
        assert_eq!(v, vec![(0..5, "A"), (5..10, "B")]);
    }

    #[test]
    fn insert_overlapping_previous_range_edge_case() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, "A");
        // Insert range that starts before but overlaps
        rm.insert_replace(5..15, "B");
        let v: Vec<_> = rm.iter().map(|(r, &s)| (r, s)).collect();
        assert_eq!(v, vec![(0..5, "A"), (5..15, "B")]);
    }

    #[test]
    fn merge_function_different_types() {
        let mut rm = RangeMap::new();
        rm.insert_replace(0..10, "prefix".to_string());
        rm.insert_with(5..15, "suffix".to_string(), |old, new| {
            format!("{}_{}", old, new)
        });
        let v: Vec<_> = rm.iter().map(|(r, s)| (r, s.as_str())).collect();
        assert_eq!(
            v,
            vec![
                (0..5, "prefix"),
                (5..10, "prefix_suffix"),
                (10..15, "suffix")
            ]
        );
    }
}
