extern crate adder_codec_rs;

use std::fs;
use adder_codec_rs::{Codec, Coord, Event};
use adder_codec_rs::raw::raw_stream::RawStream;
use rand::Rng;
use adder_codec_rs::framer::framer::SourceType::U8;

#[test]
#[should_panic]
fn test_encode_header_non_init() {
    let mut stream: RawStream = Codec::new();
    stream.encode_header(50, 100, 53000, 4000, 50000, 1);
    // stream = RawStream::new();
}

#[test]
fn test_encode_header() {
    let n = rand_u32();
    let mut stream = setup_raw_writer(n);
    cleanup_raw_writer(n, &mut stream);
}

fn setup_raw_writer(rand_num: u32) -> RawStream {
    let mut stream: RawStream = Codec::new();
    stream.open_writer("./TEST_".to_owned() + rand_num.to_string().as_str() + ".addr").expect("Couldn't open file");
    stream.encode_header(50, 100, 53000, 4000, 50000, 1);
    stream
}

fn cleanup_raw_writer(rand_num: u32, stream: &mut RawStream) {
    stream.close_writer();
    fs::remove_file("./TEST_".to_owned() + rand_num.to_string().as_str() + ".addr");  // Don't check the error
}

#[test]
#[should_panic]
fn test_encode_bad_event1() {
    let n = 0;
    let mut stream = setup_raw_writer(n);
    let event: Event = Event {
        coord: Coord {
            x: 100,
            y: 30,
            c: None
        },
        d: 5,
        delta_t: 1000
    };
    stream.encode_event(&event);
    cleanup_raw_writer(n, &mut stream)
}

#[test]
#[should_panic]
fn test_encode_bad_event2() {
    let n = 0;
    let mut stream = setup_raw_writer(n);
    let event: Event = Event {
        coord: Coord {
            x: 10,
            y: 30,
            c: Some(1)
        },
        d: 5,
        delta_t: 1000
    };
    stream.encode_event(&event);
    cleanup_raw_writer(n, &mut stream)
}

#[test]
#[should_panic]
fn test_encode_bad_event3() {
    let n = 0;
    let mut stream = setup_raw_writer(n);
    let event: Event = Event {
        coord: Coord {
            x: 10,
            y: 30,
            c: None
        },
        d: 5,
        delta_t: 1000000
    };
    stream.encode_event(&event);
    cleanup_raw_writer(n, &mut stream)
}

#[test]
fn test_encode_event() {
    let n = 0;
    let mut stream = setup_raw_writer(n);
    let event: Event = Event {
        coord: Coord {
            x: 10,
            y: 30,
            c: None
        },
        d: 5,
        delta_t: 1000
    };
    stream.encode_event(&event);
    cleanup_raw_writer(n, &mut stream)
}

#[test]
fn test_encode_events() {
    let n = 0;
    let mut stream = setup_raw_writer(n);
    let event: Event = Event {
        coord: Coord {
            x: 10,
            y: 30,
            c: None
        },
        d: 5,
        delta_t: 1000
    };
    let events = vec![event, event, event];
    stream.encode_events(&events);
    stream.flush_writer();
    cleanup_raw_writer(n, &mut stream)
}

fn setup_raw_reader(rand_num: u32, stream: &mut RawStream) {
    stream.open_reader("./TEST_".to_owned() + rand_num.to_string().as_str() + ".addr").expect("Couldn't open file");
    stream.decode_header();

}

fn rand_u32() -> u32 {
    rand::thread_rng().gen()
}

#[test]
fn read_header() {
    let n: u32 = rand::thread_rng().gen();
    let mut stream = setup_raw_writer(n);
    stream.flush_writer();
    setup_raw_reader(n, &mut stream);
    cleanup_raw_writer(n, &mut stream);
}

#[test]
fn read_event() {
    let n: u32 = rand::thread_rng().gen();
    let mut stream = setup_raw_writer(n);
    let event: Event = Event {
        coord: Coord {
            x: 10,
            y: 30,
            c: None
        },
        d: 5,
        delta_t: 1000
    };
    stream.encode_event(&event);
    stream.flush_writer();
    setup_raw_reader(n, &mut stream);
    let res = stream.decode_event();
    match res {
        Ok(decoded_event) => {
            assert_eq!(event, decoded_event);
        }
        Err(_) => {
            panic!("Couldn't decode event")
        }
    }

    cleanup_raw_writer(n, &mut stream);
}

#[test]
fn test_iter_2d() {
    use adder_codec_rs::Event;
    use adder_codec_rs::framer::array3d::{Array3D, Array3DError};
    let mut  arr: Array3D<u16> = Array3D::new(10, 10, 3);
    arr.set_at(100, 0,0,0);
    arr.set_at(250, 0,0,1);
    arr.set_at(325,0,0,2);
    for elem in &arr.iter_2d() {
        let first_sum = elem.sum::<u16>();  // Just summing the first element to show an example
        assert_eq!(first_sum, 675);
        break;
    }
    for mut elem in &arr.iter_2d_mut() {
        for i in elem {
            *i = *i + 1;
        }
        break;
    }
    for elem in &arr.iter_2d() {
        let first_sum = elem.sum::<u16>();
        assert_eq!(first_sum, 678);
        break;
    }
}
