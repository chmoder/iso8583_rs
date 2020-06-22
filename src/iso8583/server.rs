//! This module contains the implementation of a ISO server (TCP)
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::iso8583::{IsoError};
use crate::iso8583::iso_spec::{IsoMsg, Spec};
use crate::iso8583::mli::MLI;

/// This struct represents an error associated with server errors
pub struct IsoServerError {
    pub msg: String
}

/// This struct represents a IsoServer
pub struct IsoServer {
    /// The listen address for this server
    sock_addr: SocketAddr,
    pub(crate) mli: Arc<Box<dyn MLI>>,
    /// The specification associated with the server
    pub spec: &'static crate::iso8583::iso_spec::Spec,
    /// The message processor to be used to handle incoming requests
    pub(crate) msg_processor: Arc<Box<dyn MsgProcessor>>,
}

/// This trait whose implementation is used by the IsoServer to handle incoming requests
pub trait MsgProcessor: Send + Sync {
    fn process(&self, iso_server: &IsoServer, msg: &mut Vec<u8>) -> Result<(Vec<u8>, IsoMsg), IsoError>;
}

impl IsoServer {
    /// Starts the server in a separate thread
    pub fn start(&self) -> JoinHandle<()> {
        let iso_server_clone = IsoServer {
            sock_addr: self.sock_addr.clone(),
            spec: self.spec,
            mli: self.mli.clone(),
            msg_processor: self.msg_processor.clone(),
        };

        std::thread::spawn(move || {
            let listener = std::net::TcpListener::bind(iso_server_clone.sock_addr).unwrap();

            for stream in listener.incoming() {
                let client = stream.unwrap();
                debug!("Accepted new connection .. {:?}", &client.peer_addr());
                new_client(&iso_server_clone, client);
            }
        })
    }
}

/// Runs a new thread to handle a new incoming connection
fn new_client(iso_server: &IsoServer, stream_: TcpStream) {
    let iso_server_clone = IsoServer {
        sock_addr: iso_server.sock_addr.clone(),
        spec: iso_server.spec,
        mli: iso_server.mli.clone(),
        msg_processor: iso_server.msg_processor.clone(),
    };

    std::thread::spawn(move || {
        let mut buf: [u8; 512] = [0; 512];

        let mut stream = stream_;

        let mut reading_mli = true;
        let mut in_buf: Vec<u8> = Vec::with_capacity(512);
        let mut mli: u32 = 0;

        loop {
            match (&stream).read(&mut buf[..]) {
                Ok(n) => {
                    if n > 0 {
                        trace!("read {} from {}", hex::encode(&buf[0..n]), stream.peer_addr().unwrap().to_string());
                        in_buf.append(&mut buf[0..n].to_vec());


                        while in_buf.len() > 0 {
                            if reading_mli {
                                match iso_server_clone.mli.parse(&mut in_buf) {
                                    Ok(n) => {
                                        mli = n;
                                        reading_mli = false;
                                    }
                                    Err(_e) => {}
                                }
                            } else {
                                //reading data
                                if mli > 0 && in_buf.len() >= mli as usize {
                                    let data = &in_buf[0..mli as usize];
                                    debug!("received request len = {}  : data = {}", mli, hex::encode(data));

                                    match iso_server_clone.msg_processor.process(&iso_server_clone, &mut data.to_vec()) {
                                        Ok(resp) => {
                                            debug!("iso_response \n raw:: {}, \n parsed:: \n {} \n ", hex::encode(&resp.0), resp.1);


                                            match iso_server_clone.mli.create(&(resp.0).len()) {
                                                Ok(mut resp_data) => {
                                                    (&mut resp_data).write_all(resp.0.as_slice()).unwrap();
                                                    stream.write_all(resp_data.as_slice()).unwrap();
                                                }
                                                Err(e) => {
                                                    error!("failed to construct mli {}", e.msg)
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("failed to handle incoming req - {}", e.msg)
                                        }
                                    }

                                    in_buf.drain(0..mli as usize).for_each(drop);
                                    mli = 0;
                                    reading_mli = true;
                                }
                            }
                        }
                    } else {
                        //socket may have been closed??
                        info!("client socket closed : {}", stream.peer_addr().unwrap().to_string());
                        return;
                    }
                }
                Err(e) => {
                    error!("client socket_err: {} {}", stream.peer_addr().unwrap().to_string(), e.to_string());
                    return;
                }
            }
        }
    });
}

/// Returns a new ISO server on success or a IsoServer if the provided addr is incorrect
pub fn new<'a>(host_port: String, mli: Box<dyn MLI>, msg_processor: Box<dyn MsgProcessor>, spec: &'static Spec) -> Result<IsoServer, IsoServerError> {
    match host_port.to_socket_addrs() {
        Ok(mut i) => {
            match i.next() {
                Some(ip_addr) => {
                    Ok(IsoServer { sock_addr: ip_addr, spec, mli: Arc::new(mli), msg_processor: Arc::new(msg_processor) })
                }
                None => {
                    Err(IsoServerError { msg: format!("invalid host_port: {} : unresolvable?", &host_port) })
                }
            }
        }
        Err(e) => Err(IsoServerError { msg: format!("invalid host_port: {}: cause: {}", &host_port, e.to_string()) })
    }
}





