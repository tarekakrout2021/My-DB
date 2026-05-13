use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::{fmt, ptr};
use std::time::Instant;
use rayon::prelude::*;
use thread_local::ThreadLocal;
use colored::Colorize;

/// Lazy-evaluated hashtable that supports multiple values per key.
pub struct LazyMultiMap<K, V> {
    /// Vector storing all key-value entries in insertion order
    entries: Vec<LazyMultiMapEntry<K, V>>,
    /// Hash table with pointers to first entry for each bucket
    table: Vec<*mut LazyMultiMapEntry<K, V>>,
    /// Bitmask for fast modulo operations
    mask: usize,
}

/// Internal entry in the LazyMultiMap hash table.
///
/// Forms a linked list of entries that hash to the same bucket.
struct LazyMultiMapEntry<K, V> {
    /// Pointer to next entry in the same hash bucket (null if last)
    next: *mut LazyMultiMapEntry<K, V>,
    /// The key for this entry
    key: K,
    /// The value associated with this key
    val: V,
}

unsafe impl<K: Send, V: Send> Send for LazyMultiMapEntry<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for LazyMultiMapEntry<K, V> {}
unsafe impl<K: Send, V: Send> Send for LazyMultiMap<K, V> {}
unsafe impl<K: Sync, V: Sync> Sync for LazyMultiMap<K, V> {}

impl<K: Hash, V> LazyMultiMap<K, V> {
    pub fn get(&'_ self, key: K) -> LazyMultiMapEntryIterator<'_, K, V> {
        if self.table.is_empty() {
            return LazyMultiMapEntryIterator {
                ptr: ptr::null(),
                key,
                _marker: PhantomData,
            };
        }

        let hash = calculate_hash(&key);
        let idx = hash & self.mask;
        let ptr = self.table[idx] as *const LazyMultiMapEntry<K, V>;

        LazyMultiMapEntryIterator {
            ptr,
            key,
            _marker: PhantomData,
        }
    }
}

/// Builder for constructing LazyMultiMaps (single-threaded build phase).
pub struct LazyMultiMapBuilder<K, V> {
    entries: Vec<LazyMultiMapEntry<K, V>>,
}

impl<K: Hash + Send, V: Send> LazyMultiMapBuilder<K, V> {
    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    pub fn insert(&mut self, key: K, val: V) {
        self.entries.push(LazyMultiMapEntry {
            next: ptr::null_mut(),
            key,
            val,
        });
    }

    pub fn finalize(mut self) -> LazyMultiMap<K, V>
    where
        K: Sync,
        V: Sync,
    {
        build_table_from_entries_parallel(&mut self.entries)
    }
}

impl<K: Hash + Send, V: Send> Default for LazyMultiMapBuilder<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over values in a LazyMultiMap for a specific key.
pub struct LazyMultiMapEntryIterator<'a, K: 'a, V: 'a> {
    ptr: *const LazyMultiMapEntry<K, V>,
    key: K,
    _marker: PhantomData<&'a K>,
}

impl<'a, K: PartialEq, V> Iterator for LazyMultiMapEntryIterator<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.ptr.is_null() {
            let entry = unsafe { &*self.ptr };
            self.ptr = entry.next;

            if entry.key == self.key {
                return Some(&entry.val);
            }
        }
        None
    }
}

/// Calculates hash value for a given key.
fn calculate_hash<T: Hash>(t: &T) -> usize {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish() as usize
}

impl<K: fmt::Debug + Hash + PartialEq, V: fmt::Debug> fmt::Debug for LazyMultiMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();

        for (bucket_idx, &head_ptr) in self.table.iter().enumerate() {
            if head_ptr.is_null() {
                continue;
            }

            let mut values = Vec::new();
            let mut p = head_ptr;

            while !p.is_null() {
                let entry = unsafe { &*p };
                values.push((&entry.key, &entry.val));
                p = entry.next;
            }

            map.entry(&bucket_idx, &values);
        }

        map.finish()
    }
}

pub struct LazyMultiMapParBuilder<K: Send, V: Send> {
    locals: ThreadLocal<RefCell<Vec<LazyMultiMapEntry<K, V>>>>,
}

impl<K: Hash + Send, V: Send> LazyMultiMapParBuilder<K, V> {
    pub fn new() -> Self {
        Self {
            locals: ThreadLocal::new(),
        }
    }

    /// Thread-safe insert: buffers into current thread's local Vec.
    pub fn insert(&self, key: K, val: V) {
        let cell = self.locals.get_or(|| RefCell::new(Vec::new()));
        cell.borrow_mut().push(LazyMultiMapEntry {
            next: ptr::null_mut(),
            key,
            val,
        });
    }
    /// Finalize by merging all thread-local buffers and building the hash table in parallel.
    pub fn finalize(self) -> LazyMultiMap<K, V>
    where
        K: Sync,
        V: Sync,
    {
        //let start = Instant::now();
        let mut entries = Vec::new();
        for cell in self.locals.into_iter() {
            entries.extend(cell.into_inner());
        }
        let ret = build_table_from_entries_parallel(&mut entries);
        // let total_time = start.elapsed();
        // println!(
        //     "{}",
        //     format!("Total Building time : {:?}", total_time).green()
        // );
        ret
    }
}

impl<K: Hash + Send, V: Send> Default for LazyMultiMapParBuilder<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

fn build_table_from_entries_parallel<K: Hash + Sync + Send, V: Sync + Send>(
    entries: &mut Vec<LazyMultiMapEntry<K, V>>,
) -> LazyMultiMap<K, V> {
    let num_entries = entries.len();

    let table_size = if num_entries == 0 {
        1
    } else {
        (num_entries * 2).next_power_of_two()
    };

    let mask = table_size - 1;

    // Atomic heads per bucket
    let table: Vec<AtomicPtr<LazyMultiMapEntry<K, V>>> = (0..table_size)
        .map(|_| AtomicPtr::new(ptr::null_mut()))
        .collect();

    entries
        .par_iter_mut()
        .for_each(|entry: &mut LazyMultiMapEntry<K, V>| {
            let hash = calculate_hash(&entry.key);
            let idx = hash & mask;

            let entry_ptr: *mut LazyMultiMapEntry<K, V> = entry as *mut _;

            loop {
                let head = table[idx].load(Ordering::Relaxed);

                entry.next = head;

                // CAS to update head to new entry, retry if head changed in the meantime
                match table[idx].compare_exchange_weak(
                    head,
                    entry_ptr,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(_) => continue,
                }
            }
        });
    let table_plain: Vec<*mut LazyMultiMapEntry<K, V>> =
        table.into_iter().map(|a| a.into_inner()).collect();

    LazyMultiMap {
        entries: std::mem::take(entries),
        table: table_plain,
        mask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn dbg_sequential_builder() {
        let mut builder = LazyMultiMapBuilder::new();
        for key in (0..5).step_by(2) {
            builder.insert(key, key + 1);
            builder.insert(key, key + 2);
        }
        let map = builder.finalize();
        println!("{:?}", map);
    }

    #[test]
    fn test_map_sequential_builder_parallel_finalize() {
        let mut builder = LazyMultiMapBuilder::new();
        for key in (0..100).step_by(2) {
            builder.insert(key, key + 1);
        }
        let map = builder.finalize();

        for key in (0..100).step_by(2) {
            assert_eq!(map.get(key).collect::<Vec<_>>(), [&(key + 1)]);
        }

        for key in (1..999).step_by(2) {
            assert_eq!(map.get(key).count(), 0);
        }
    }

    #[test]
    fn test_parallel_build_parallel_finalize() {
        let builder: LazyMultiMapParBuilder<i32, i32> = LazyMultiMapParBuilder::new();

        (0..10_000i32).into_par_iter().for_each(|k| {
            if k % 2 == 0 {
                builder.insert(k, k + 1);
            }
        });

        let map = builder.finalize();

        for k in (0..10_000i32).step_by(2) {
            let got = map.get(k).collect::<Vec<_>>();
            assert_eq!(got, [&(k + 1)]);
        }

        for k in (1..10_000i32).step_by(2) {
            assert_eq!(map.get(k).count(), 0);
        }
    }

    #[test]
    fn test_shared_map_parallel_probe() {
        let builder: LazyMultiMapParBuilder<i32, i32> = LazyMultiMapParBuilder::new();

        (0..10_000i32).into_par_iter().for_each(|k| {
            if k % 2 == 0 {
                builder.insert(k, k + 1);
                builder.insert(k, k + 2);
            }
        });

        let map = Arc::new(builder.finalize());

        (0..10_000i32).into_par_iter().for_each(|k| {
            if k % 2 == 0 {
                let mut vals = map.get(k).copied().collect::<Vec<_>>();
                vals.sort();
                assert_eq!(vals, vec![k + 1, k + 2]);
            } else {
                assert_eq!(map.get(k).count(), 0);
            }
        });
    }
}
