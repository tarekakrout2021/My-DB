use std::alloc::{Layout, alloc, dealloc};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::io::Write;

use crate::types::{Tag, Type};
use crate::util::io::Output;

/// Variable-length text with inline optimization: stores up to 12 bytes inline, larger strings on heap with 4-byte prefix.
#[derive(Debug)]
#[repr(C)]
pub struct Text {
    pub len: u32,
    pub data: [u8; 12],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TextView {
    pub len: u32,
    pub data: [u8; 12],
}

impl TextView {
    pub fn as_str(&self) -> &str {
        let bytes = if self.len <= 12 {
            &self.data[..self.len as usize]
        } else {
            let ptr = u64::from_le_bytes(self.data[4..].try_into().unwrap()) as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, self.len as usize) }
        };
        std::str::from_utf8(bytes).unwrap()
    }
}

impl Text {
    /// Parses a string into a Text.
    pub fn input(r#in: &str, r#type: Type) -> Result<Self, Box<dyn Error>> {
        assert!(matches!(
            r#type.r#type(),
            Tag::Char | Tag::VarChar | Tag::Text
        ));

        let len = r#in.len();
        let mut data = [0; 12];

        if len <= 12 {
            data[..len].copy_from_slice(&r#in.as_bytes()[..len]);
        } else {
            if len > u32::MAX as usize {
                panic!("maximum length exceeded for text type");
            }

            // prefix
            data[..4].copy_from_slice(&r#in.as_bytes()[..4]);
            // copy in to new location
            let layout = Layout::array::<u8>(len)?;
            let ptr = unsafe { alloc(layout) };
            unsafe { std::ptr::copy_nonoverlapping(r#in.as_ptr(), ptr, len) };
            // store ptr to new location
            data[4..].copy_from_slice(&(ptr as u64).to_le_bytes());
        }

        Ok(Self {
            len: len as u32,
            data,
        })
    }

    /// Writes a Text to the output writer.
    pub fn output(writer: &mut Output, r#type: Type, out: &Self) -> std::io::Result<()> {
        assert!(matches!(
            r#type.r#type(),
            Tag::Char | Tag::VarChar | Tag::Text
        ));

        let slice = if out.len <= 12 {
            &out.data[..out.len as usize]
        } else {
            let ptr = u64::from_le_bytes(out.data[4..].try_into().unwrap()) as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, out.len as usize) }
        };
        let s = std::str::from_utf8(slice).unwrap();

        write!(writer, "{s}")
    }

    /// Returns a slice of the text data, handling both inline and heap-allocated cases.
    pub fn as_slice(&self) -> &[u8] {
        if self.len <= 12 {
            &self.data[..self.len as usize]
        } else {
            let ptr = u64::from_le_bytes(self.data[4..].try_into().unwrap()) as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, self.len as usize) }
        }
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        self.as_slice() == other.as_slice()
    }
}

impl Clone for Text {
    fn clone(&self) -> Self {
        if self.len <= 12 {
            Self {
                len: self.len.clone(),
                data: self.data.clone(),
            }
        } else {
            let mut data = [0; 12];
            data[..4].copy_from_slice(&self.as_slice()[..4]);
            // copy in to new location
            let layout = Layout::array::<u8>(self.len as usize).unwrap();
            let ptr = unsafe { alloc(layout) };
            unsafe {
                std::ptr::copy_nonoverlapping(self.as_slice().as_ptr(), ptr, self.len as usize)
            };
            // store ptr to new location
            data[4..].copy_from_slice(&(ptr as u64).to_le_bytes());
            Self {
                len: self.len.clone(),
                data,
            }
        }
    }
}
impl Eq for Text {}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        if self.len > 12 {
            let layout = Layout::array::<u8>(self.len as usize).unwrap();
            let ptr = u64::from_le_bytes(self.data[4..].try_into().unwrap()) as *mut u8;
            unsafe { dealloc(ptr, layout) };
        }
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // This is basically your Text::output logic, but without Type.
        let slice = if self.len <= 12 {
            &self.data[..self.len as usize]
        } else {
            let ptr = u64::from_le_bytes(self.data[4..].try_into().unwrap()) as *const u8;
            unsafe { std::slice::from_raw_parts(ptr, self.len as usize) }
        };
        let s = std::str::from_utf8(slice).unwrap();
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufWriter;

    macro_rules! output_eq {
        ($l:expr, $t:expr, $r:expr) => {
            let mut buf = BufWriter::new(Vec::new());
            let mut out = Output::new(&mut buf);
            Text::output(&mut out, $t, &$l).unwrap();
            let bytes = buf.into_inner().unwrap();
            let out = String::from_utf8(bytes).unwrap();
            assert_eq!(out, $r.to_string());
        };
    }

    #[test]
    fn test_input() {
        let c = Text::input("hello world!", Type::new_text()).unwrap();
        output_eq!(c, Type::new_text(), "hello world!");

        let c = Text::input("hello world!     ", Type::new_char(42)).unwrap();
        output_eq!(c, Type::new_char(42), "hello world!     ");

        let c = Text::input("hello", Type::new_varchar(8)).unwrap();
        output_eq!(c, Type::new_varchar(8), "hello");

        let c = Text::input("", Type::new_text()).unwrap();
        output_eq!(c, Type::new_text(), "");
    }

    #[test]
    fn test_eq() {
        // Test equality for inline strings (≤12 bytes)
        let t1 = Text::input("hello", Type::new_text()).unwrap();
        let t2 = Text::input("hello", Type::new_text()).unwrap();
        let t3 = Text::input("world", Type::new_text()).unwrap();
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);

        // Test equality for heap-allocated strings (>12 bytes)
        let t4 = Text::input("hello world this is a long string", Type::new_text()).unwrap();
        let t5 = Text::input("hello world this is a long string", Type::new_text()).unwrap();
        let t6 = Text::input("hello world this is different text", Type::new_text()).unwrap();
        assert_eq!(t4, t5);
        assert_ne!(t4, t6);
        // Test equality between inline and heap
        let t7 = Text::input("short", Type::new_text()).unwrap();
        let t8 = Text::input("this is a much longer string", Type::new_text()).unwrap();
        assert_ne!(t7, t8);

        // Test empty string
        let t9 = Text::input("", Type::new_text()).unwrap();
        let t10 = Text::input("", Type::new_text()).unwrap();
        assert_eq!(t9, t10);
    }

    #[test]
    fn test_ord() {
        // Test ordering for inline strings
        let t1 = Text::input("apple", Type::new_text()).unwrap();
        let t2 = Text::input("banana", Type::new_text()).unwrap();
        let t3 = Text::input("apple", Type::new_text()).unwrap();
        assert!(t1 < t2);
        assert!(t2 > t1);
        assert!(t1 <= t3);
        assert!(t1 >= t3);

        // Test ordering for heap-allocated strings
        let t4 = Text::input("this is a long string alpha", Type::new_text()).unwrap();
        let t5 = Text::input("this is a long string beta", Type::new_text()).unwrap();
        assert!(t4 < t5);
        assert!(t5 > t4);

        // Test ordering between inline and heap
        let t6 = Text::input("zzz", Type::new_text()).unwrap();
        let t7 = Text::input("aaa this is a longer string", Type::new_text()).unwrap();
        assert!(t7 < t6);

        // Test empty string ordering
        let t8 = Text::input("", Type::new_text()).unwrap();
        let t9 = Text::input("a", Type::new_text()).unwrap();
        assert!(t8 < t9);
    }

    #[test]
    fn test_hash() {
        use std::collections::{HashMap, HashSet};
        use std::hash::{DefaultHasher, Hasher};

        // Test that equal texts have the same hash
        let t1 = Text::input("hello", Type::new_text()).unwrap();
        let t2 = Text::input("hello", Type::new_text()).unwrap();

        let mut hasher1 = DefaultHasher::new();
        t1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        t2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2, "Equal texts should have equal hashes");

        // Test that different texts have different hashes (usually)
        let t3 = Text::input("world", Type::new_text()).unwrap();
        let mut hasher3 = DefaultHasher::new();
        t3.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        assert_ne!(
            hash1, hash3,
            "Different texts should usually have different hashes"
        );

        // Test with heap-allocated strings
        let t4 = Text::input("this is a long string for testing", Type::new_text()).unwrap();
        let t5 = Text::input("this is a long string for testing", Type::new_text()).unwrap();

        let mut hasher4 = DefaultHasher::new();
        t4.hash(&mut hasher4);
        let hash4 = hasher4.finish();

        let mut hasher5 = DefaultHasher::new();
        t5.hash(&mut hasher5);
        let hash5 = hasher5.finish();

        assert_eq!(
            hash4, hash5,
            "Equal heap-allocated texts should have equal hashes"
        );

        // Test using Text in a HashSet
        let mut set = HashSet::new();
        set.insert(Text::input("apple", Type::new_text()).unwrap());
        set.insert(Text::input("banana", Type::new_text()).unwrap());
        set.insert(Text::input("apple", Type::new_text()).unwrap()); // duplicate

        assert_eq!(set.len(), 2, "HashSet should contain 2 unique elements");

        // Test using Text in a HashMap
        let mut map = HashMap::new();
        map.insert(Text::input("key1", Type::new_text()).unwrap(), 42);
        map.insert(Text::input("key2", Type::new_text()).unwrap(), 100);

        let lookup_key = Text::input("key1", Type::new_text()).unwrap();
        assert_eq!(
            map.get(&lookup_key),
            Some(&42),
            "Should be able to lookup values in HashMap"
        );
    }
    #[test]
    fn test_clone() {
        let cloned: Text;
        {
            let c = Text::input("hello cloned text!", Type::new_text()).unwrap();
            output_eq!(c, Type::new_text(), "hello cloned text!");
            cloned = c.clone();

            output_eq!(cloned, Type::new_text(), "hello cloned text!");
        }

        output_eq!(cloned, Type::new_text(), "hello cloned text!");
    }
}
