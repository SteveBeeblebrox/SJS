use std::hash::Hasher;

pub fn hash<T: AsRef<[u8]>>(v: T) -> String {
    let mut hasher = twox_hash::XxHash64::default();
    hasher.write(v.as_ref());
    return format!("{:x}", hasher.finish());
}