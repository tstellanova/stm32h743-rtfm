
use embedded_hal::blocking;


// the address normally used by our BNO080 breakout
pub const DEFAULT_ADDRESS: u8 =  0x4B;
// alternate address for BNO080
pub const ALTERNATE_ADDRESS: u8 =  0x4A;
pub const I2C_BUFFER_LENGTH: usize =  32;
// length of packet headers
pub const PACKET_HEADER_LENGTH: usize = 4;

//channels
pub const  CHANNEL_COMMAND: usize = 0;
pub const  CHANNEL_EXECUTABLE: usize = 1;
pub const  CHANNEL_CONTROL: usize = 2;
pub const  CHANNEL_REPORTS: usize = 3;
pub const  CHANNEL_WAKE_REPORTS: usize = 4;
pub const  CHANNEL_GYRO: usize = 5;






//trait NewTrait: Clone + Default + OtherTraits {}
//impl<T> NewTrait for T where T: Clone + Default + OtherTraits {}

pub trait Porty: blocking::i2c::Read + blocking::i2c::Write {}


pub struct PortDriver<T: Porty>  {
    // each communication channel with the device has its own sequence number
    sequence_numbers: [u8; 6],
    send_buf: [u8; 64],
    recv_buf: [u8; 256],
    address: u8,
    port:  T,
}


impl<T:Porty> PortDriver<T>  {

    pub fn new(port: T, address: u8) -> Self {
        PortDriver {
            sequence_numbers:  [0; 6],
            send_buf: [0; 64],
            recv_buf: [0; 256],
            address: address,
            port: port,
        }
    }

    // Send a standard header followed by the data provided
    // TODO proper error handling (Eg on no ACK from sensor)
    pub fn send_packet(&mut self, channel: usize, data: &[u8]) {
        let packet_length = data.len() + PACKET_HEADER_LENGTH;
        let packet_header = [
            (packet_length & 0xFF) as u8, //LSB
            (packet_length >> 8) as u8, //MSB
            channel as u8,
            self.sequence_numbers[channel]
        ];

        self.send_buf.copy_from_slice(packet_header.as_ref());
        self.send_buf[PACKET_HEADER_LENGTH..].clone_from_slice(data);

        self.sequence_numbers[channel] += 1;

        self.port.write(self.address, self.send_buf.as_ref()).ok();
    }

    pub fn receive_packet(&mut self) {
        let mut header_data:[u8; PACKET_HEADER_LENGTH] = [0,0,0,0];
        //read packet header
        if self.port.read(self.address, &mut header_data).is_ok() {
            //got a packet
            let packet_len_lsb = header_data[0];
            let packet_len_msb =  header_data[1];
            let _chan_num =  header_data[2]; //TODO always CHANNEL_REPORTS ?
            let  _seq_num =  header_data[3];

            let mut packet_length:usize = (packet_len_msb << 8 | packet_len_lsb) as usize;
            if packet_length > 0 {
                //continuation bit, MS, is 1<<15 = 32768
                packet_length = packet_length & (!32768); //clear continuation bit (MS)
                packet_length -= 4; //remove header length
                self.port.read(self.address, &mut self.recv_buf[..packet_length] ).ok();
            }
        }
    }

    pub fn reset_sensor(&mut self) {
        let data:[u8; 1] = [1];
        self.send_packet(CHANNEL_EXECUTABLE, data.as_ref());
        //TODO read any garbage data
    }

}







