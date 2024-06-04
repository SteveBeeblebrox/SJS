use std::hash::{Hash,Hasher};

pub fn hash<T: AsRef<&[u8]>>(v: T) -> String {
    let mut hasher = twox_hash::XxHash64::default();
    hasher.write(v);
    return format!("{:x}", hasher.finish());
}