#[cfg(test)]
mod tests {
    use std::net::TcpStream;
    use std::io::{Write, Read, BufReader, Error};
    use std::time::Duration;
    use byteorder::{WriteBytesExt, ReadBytesExt};
    use byteorder::ByteOrder;
    use crate::iso8583::iso_spec;
    use crate::iso8583::server::IsoServer;

    #[test]
    fn test_server() -> Result<(), ()> {
        let _ = simplelog::SimpleLogger::init(simplelog::LevelFilter::Debug, simplelog::Config::default());

        let iso_spec = crate::iso8583::iso_spec::spec("SampleSpec");

        info!("starting iso server for spec {} at port {}", iso_spec.name(), 6666);
        let server: IsoServer = match crate::iso8583::server::new("localhost:6666".to_string(), iso_spec) {
            Ok(server) => {
                server
            }
            Err(e) => {
                panic!(e)
            }
        };
        server.start().join();
        Ok(())
    }


    #[test]
    fn test_send_recv_iso() -> Result<(), Error> {
        let mut raw_msg: Vec<u8> = Vec::new();

        //make space for mli (swapped later)
        raw_msg.push(0);
        raw_msg.push(0);

        // message type
        "1100".as_bytes().read_to_end(&mut raw_msg);

        let mut bmp = crate::iso8583::bitmap::new_bmp(0, 0, 0);
        bmp.set_on(2);
        bmp.set_on(3);
        bmp.set_on(4);
        bmp.set_on(11);
        bmp.set_on(14);
        bmp.set_on(52);
        bmp.set_on(96);
        bmp.set_on(160);

        //bitmap
        raw_msg.write_all(hex::decode(bmp.hex_string()).expect("failed to decode bmp").as_slice());

        //pan - with length indicator and data
        "12".as_bytes().read_to_end(&mut raw_msg);
        "123456789101".as_bytes().read_to_end(&mut raw_msg);

        //proc code
        "004000".as_bytes().read_to_end(&mut raw_msg);

        //amount
        "000000000199".as_bytes().read_to_end(&mut raw_msg);

        //stan
        "779581".as_bytes().read_to_end(&mut raw_msg);

        //expiration date
        "2204".as_bytes().read_to_end(&mut raw_msg);

        if bmp.is_on(52){
            "0102030405060708".as_bytes().read_to_end(&mut raw_msg);
        }

        //bit 96
        "1234".as_bytes().read_to_end(&mut raw_msg);

        //bit 160
        "8888".as_bytes().read_to_end(&mut raw_msg);


        let mut mli: [u8; 2] = [0; 2];
        byteorder::BigEndian::write_u16(&mut mli[..], raw_msg.len() as u16 - 2);

        std::mem::swap(&mut mli[0], &mut raw_msg[0]);
        std::mem::swap(&mut mli[1], &mut raw_msg[1]);

        println!("raw iso msg = {}", hex::encode(raw_msg.as_slice()));

        let mut client = TcpStream::connect("localhost:6666")?;

        client.write_all(raw_msg.as_slice())?;
        client.flush();

        // read the response

        let mut reader = BufReader::new(&mut client);
        let len = reader.read_u16::<byteorder::BigEndian>().unwrap();


        let mut out_buf = vec![0; len as usize];

        match reader.read_exact(&mut out_buf[..]) {
            Ok(()) => {
                println!("received response:  {:?} with  {} bytes.", hex::encode(&out_buf), len);
                match iso_spec::spec("SampleSpec").parse(&mut out_buf) {
                    Ok(resp_iso_msg) => {
                        println!("parsed iso-response \"{}\" \n {} \n", resp_iso_msg.msg.name(), resp_iso_msg);
                    }
                    Err(e) => panic!(e.msg)
                }
            }
            Err(e) => {
                panic!(e)
            }
        }
        Ok(())
    }
}
