#![no_std]

use crc16::{State, MODBUS};

#[derive(Debug, Clone, Copy)]
pub struct Packet {
    pub sync: u16,
    pub command: u8,
    pub length: u8,
    pub payload: Option<[u8; 255]>,
    pub checksum: u16
}

impl Packet {
    // TODO: come up with a better implementation
    pub fn write_bytes(&self, buffer: &mut [u8]) -> u32 {

        let mut i = 0;
        buffer[i] = self.sync as u8;
        i = i + 1;
        buffer[i] = (self.sync >> 8) as u8;
        i = i + 1;
        buffer[i] = self.command;
        i = i + 1;
        buffer[i] = self.length;
        i = i + 1;

        match self.payload {
            None => (),
            Some(payload) => {
                for j in 0..(self.length as usize) {
                    buffer[i] = payload[j];
                    i = i + 1;
                }
            }
        };

        buffer[i] = self.checksum as u8;
        i = i + 1;
        buffer[i] = (self.checksum >> 8) as u8;

        (self.length as u32) + 6
    }

    /// Computes checksum for the packet and populates the `checksum` field accordingly
    fn compute_checksum(&self) -> u16 {
        let mut state = State::<MODBUS>::new();

        state.update(&self.sync.to_le_bytes());
        state.update(&self.command.to_le_bytes());
        state.update(&self.length.to_le_bytes());
        match self.payload {
            None => (),
            Some(payload) => {
                let payload_len = self.length as usize;
                state.update(&payload[0..payload_len]); // FIXME: something wrong with this?
            }
        };

        state.get()
    }

    /// Create an empty packet with only the sync field populated
    pub const fn new() -> Self {
        Packet {
            sync: 0x55AA,
            command: 0,
            length: 0,
            payload: None,
            checksum: 0,
        }
    }

    pub fn from(command: u8, payload: Option<&[u8]>) -> Self {
        let mut packet = Packet::new();
        packet.command = command;
        if let Some(payload_src) = payload {
            assert!(payload_src.len() <= 255);

            packet.length = payload_src.len() as u8;

            let mut payload_new = [0u8; 255];
            for (src, dest) in payload_src.iter().zip(payload_new.iter_mut()) {
                *dest = *src;
            }
            packet.payload = Some(payload_new);
        };

        packet.checksum = packet.compute_checksum();
        packet
    }

    pub fn validate(self) -> Result<Self, ()> {
        if (self.compute_checksum() == self.checksum) {
            Ok(self)
        } else {
            Err(())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_is_correct() {
        let payload = b"Hello, world!";
        let command = 0x03;
        let packet = Packet::from(command, Some(&payload[..]));
        assert_eq!(packet.command, command);

        match packet.payload {
            None => (),
            Some(p) => {
                for i in 0..payload.len() {
                    assert_eq!(p[i], payload[i]);
                }
                for i in payload.len()..255 {
                    assert_eq!(p[i], 0);
                }
            }
        }

    }

    #[test]
    #[should_panic]
    fn new_disallows_large_payloads() {
        let too_large = [3; 256];
        let command = 0x05;
        let packet = Packet::from(command, Some(&too_large[..]));
    }

    #[test]
    fn large_payload() {
        const BUF_SIZE: usize = 261;
        let mut buffer: [u8; BUF_SIZE] = [0; BUF_SIZE];

        let mut payload = [0; 255];
        payload[0] = 1;
        payload[1] = 2;
        payload[2] = 3;

        let packet_struct = Packet::from(0x01, Some(&payload));
        let len = packet_struct.write_bytes(&mut buffer);

        let correct = [0xAA, 0x55, 0x01, 255, 1, 2, 3, 0];

        assert_eq!(len, 261);
        assert_eq!(&buffer[..8], &correct)
    }

    #[test]
    fn validate() {
        let mut packet = Packet::from(8, Some(b"Moi :DD"));
        packet = match packet.validate() {
            Ok(p) => p, // should work
            Err(e) => panic!("checksum should be valid"),
        };

        packet.checksum += 1;
        match packet.validate() {
            Ok(p) => panic!("should return error type"),
            Err(e) => (),
        }
    }

    #[test]
    fn compute_checksum() {
        let example_bytes = [0xAA, 0x55, 0x01, 0x03, 0xAA, 0xAA, 0xAA];
        let mut example_checksum: u16 = State::<MODBUS>::calculate(&example_bytes[..]);

        let command = 0x01;
        let payload = [0xAA, 0xAA, 0xAA];
        let p = Packet::from(command, Some(&payload[..]));

        assert_eq!(p.checksum, example_checksum);
    }

    #[test]
    fn no_payload() {
        let command = 0x08;
        let p = Packet::from(command, None);

        assert_eq!(p.length, 0);
    }

}
