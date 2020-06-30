//https://www.eftlab.com/knowledge-base/261-complete-list-of-pin-blocks-in-payments/

use rand;
use super::rand::Rng;
use generic_array::GenericArray;
use des::block_cipher::NewBlockCipher;
use des::block_cipher::BlockCipher;

#[derive(Debug)]
pub enum PinFormat {
    ISO0,
    //ANSI X9.8, ECI-4
    ISO1,
    ISO2,
    ISO3,
    ISO4,
}

pub struct PinError {
    msg: String
}

pub fn generate_pin_block<'a>(fmt: &'a PinFormat, c_pin: &str, pan: &str, key: &str) -> Result<Vec<u8>, PinError> {
    match fmt {
        ISO0 => {
            let mut tmp = format!("0{}{}", c_pin.len(), c_pin);
            pad_8(&mut tmp);
            println!("with padding {}", tmp);

            //rightmost 12 not including check digit
            let mut tmp2 = String::from("0000");
            tmp2.push_str(&pan[pan.len() - 13..pan.len() - 1]);

            let res = hex::decode(tmp).unwrap().iter().zip(hex::decode(tmp2).unwrap().iter()).map(|f| f.0 ^ f.1).collect::<Vec<u8>>();


            println!("res {}", hex::encode(res.as_slice()));
            let block_cipher = des::TdesEde2::new(GenericArray::from_slice(hex::decode(key).unwrap().as_slice()));
            let mut inp_block = GenericArray::clone_from_slice(res.as_slice());
            block_cipher.encrypt_block(&mut inp_block);
            Ok(inp_block.to_vec())
        }
        _ => {
            Err(PinError { msg: format!("{:?} is not supported yet.", fmt) })
        }
    }
}


//pad the hex string 'data' to be full 8 bytes
fn pad_8(data: &mut String) {
    let mut padding: [u8; 8] = rand::thread_rng().gen();
    //data.push_str(hex::encode(padding).as_str());
    data.push_str("FFFFFFFFFFFFFFFF");
    data.truncate(16);
}


#[cfg(test)]
mod tests {
    use crate::crypto::pin::generate_pin_block;
    use crate::crypto::pin::PinFormat::ISO0;

    #[test]
    fn test_iso0() {
        match generate_pin_block(&ISO0, "1234", "4111111111111111", "e0f4543f3e2a2c5ffc7e5e5a222e3e4d") {
            Ok(p) => {
                assert_eq!(hex::encode(p), "6042012526a9c2e0");
            }
            Err(e) => {
                assert!(false, e.msg.to_string());
            }
        }
    }
}