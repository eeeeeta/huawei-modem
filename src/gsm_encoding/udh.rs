use std::convert::TryFrom;
use errors::*;

#[derive(Debug, Clone)]
pub struct UdhComponent {
    pub id: u8,
    pub data: Vec<u8>
}
#[derive(Debug, Clone)]
pub struct UserDataHeader {
    pub components: Vec<UdhComponent>
}
impl UserDataHeader {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut ret = vec![];
        for comp in self.components.iter() {
            ret.push(comp.id);
            ret.push(comp.data.len() as u8);
            ret.extend(comp.data.clone());
        }
        let len = ret.len() as u8;
        ret.insert(0, len);
        ret
    }
}
impl<'a> TryFrom<&'a [u8]> for UserDataHeader {
    type Error = HuaweiError;
    /// Accepts a UDH *without* the UDH Length octet at the start.
    fn try_from(b: &[u8]) -> HuaweiResult<Self> {
        let mut offset = 0;
        let mut ret = vec![];
        loop {
            if b.get(offset).is_none() {
                break;
            }
            let id = b[offset];
            offset += 1;
            check_offset!(b, offset, "UDH component length");
            let len = b[offset];
            let end = offset + len as usize + 1;
            offset += 1;
            let o = end - 1;
            check_offset!(b, o, "UDH component data");
            let data = b[offset..end].to_owned();
            offset = end;
            ret.push(UdhComponent { id, data });
        }
        Ok(UserDataHeader {
            components: ret
        })
    }
}

