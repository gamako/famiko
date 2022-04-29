
pub fn dump_bytes(a : &[u8] ) -> String {
    a
        .iter()
        .map(|x| { format!("{:02X}", *x) })
        .collect::<Vec<_>>()
        .join(" ")
}
