// Copyright 2016 James Bendig. See the COPYRIGHT file at the top-level
// directory of this distribution.
//
// Licensed under:
//   the MIT license
//     <LICENSE-MIT or https://opensource.org/licenses/MIT>
//   or the Apache License, Version 2.0
//     <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0>,
// at your option. This file may not be copied, modified, or distributed
// except according to those terms.

pub mod fields;
pub mod messages;

use std::collections::{HashMap,HashSet};

use fixt::message::FIXTMessage;

#[macro_export]
macro_rules! define_dictionary {
    ( $( $msg:ty : $msg_enum:ident ),* $(),* ) => {
        fn build_dictionary() -> std::collections::HashMap<&'static [u8],Box<$crate::fixt::message::FIXTMessage + Send>> {
            let mut message_dictionary: std::collections::HashMap<&'static [u8],Box<$crate::fixt::message::FIXTMessage + Send>> = std::collections::HashMap::new();
            $( message_dictionary.insert(<$msg as $crate::message::MessageDetails>::msg_type(),Box::new(<$msg as Default>::default())); )*

            message_dictionary
        }

        #[allow(dead_code)]
        enum MessageEnum
        {
            $( $msg_enum($msg), )*
        };

        #[allow(dead_code)]
        fn message_to_enum(message: &$crate::fixt::message::FIXTMessage) -> MessageEnum {
            if false {
            }
            $( else if message.as_any().is::<$msg>() {
                //TODO: Avoid the clone.
                return MessageEnum::$msg_enum(message.as_any().downcast_ref::<$msg>().unwrap().clone());
            } )*

            panic!("Unsupported message");
        }
    };
}

pub trait CloneDictionary {
    fn clone(&self) -> HashMap<&'static [u8],Box<FIXTMessage + Send>>;
}

impl CloneDictionary for HashMap<&'static [u8],Box<FIXTMessage + Send>> {
    fn clone(&self) -> HashMap<&'static [u8],Box<FIXTMessage + Send>> {
        //TODO: This function wastes a lot of time and memory. Probably better to change Parser
        //so it isn't needed.

        let mut result = HashMap::<&'static [u8],Box<FIXTMessage + Send>>::new();
        for (key,value) in self {
            result.insert(key,FIXTMessage::new_into_box(&**value));
        }

        result
    }
}

pub fn standard_msg_types() -> HashSet<&'static [u8]> {
    let mut result: HashSet<&'static [u8]> = HashSet::with_capacity(118 * 2);

    //List taken from FIX5SP2 Volume 6, page 13.
    result.insert(b"0");
    result.insert(b"1");
    result.insert(b"2");
    result.insert(b"3");
    result.insert(b"4");
    result.insert(b"5");
    result.insert(b"6");
    result.insert(b"7");
    result.insert(b"8");
    result.insert(b"9");
    result.insert(b"A");
    result.insert(b"AA");
    result.insert(b"AB");
    result.insert(b"AC");
    result.insert(b"AD");
    result.insert(b"AE");
    result.insert(b"AF");
    result.insert(b"AG");
    result.insert(b"AH");
    result.insert(b"AI");
    result.insert(b"AJ");
    result.insert(b"AK");
    result.insert(b"AL");
    result.insert(b"AM");
    result.insert(b"AN");
    result.insert(b"AO");
    result.insert(b"AP");
    result.insert(b"AQ");
    result.insert(b"AR");
    result.insert(b"AS");
    result.insert(b"AT");
    result.insert(b"AU");
    result.insert(b"AV");
    result.insert(b"AW");
    result.insert(b"AX");
    result.insert(b"AY");
    result.insert(b"AZ");
    result.insert(b"B");
    result.insert(b"BA");
    result.insert(b"BB");
    result.insert(b"BC");
    result.insert(b"BD");
    result.insert(b"BE");
    result.insert(b"BF");
    result.insert(b"BG");
    result.insert(b"BH");
    result.insert(b"BI");
    result.insert(b"BJ");
    result.insert(b"BK");
    result.insert(b"BL");
    result.insert(b"BM");
    result.insert(b"BN");
    result.insert(b"BO");
    result.insert(b"BP");
    result.insert(b"BQ");
    result.insert(b"BR");
    result.insert(b"BS");
    result.insert(b"BT");
    result.insert(b"BU");
    result.insert(b"BV");
    result.insert(b"BW");
    result.insert(b"BX");
    result.insert(b"BY");
    result.insert(b"BZ");
    result.insert(b"C");
    result.insert(b"CA");
    result.insert(b"CB");
    result.insert(b"CC");
    result.insert(b"CD");
    result.insert(b"CD");
    result.insert(b"CF");
    result.insert(b"CG");
    result.insert(b"D");
    result.insert(b"E");
    result.insert(b"F");
    result.insert(b"G");
    result.insert(b"H");
    result.insert(b"J");
    result.insert(b"K");
    result.insert(b"L");
    result.insert(b"M");
    result.insert(b"N");
    result.insert(b"P");
    result.insert(b"Q");
    result.insert(b"R");
    result.insert(b"S");
    result.insert(b"T");
    result.insert(b"V");
    result.insert(b"W");
    result.insert(b"X");
    result.insert(b"Y");
    result.insert(b"Z");
    result.insert(b"a");
    result.insert(b"b");
    result.insert(b"c");
    result.insert(b"d");
    result.insert(b"e");
    result.insert(b"f");
    result.insert(b"g");
    result.insert(b"h");
    result.insert(b"i");
    result.insert(b"j");
    result.insert(b"k");
    result.insert(b"l");
    result.insert(b"m");
    result.insert(b"n");
    result.insert(b"o");
    result.insert(b"p");
    result.insert(b"q");
    result.insert(b"r");
    result.insert(b"s");
    result.insert(b"t");
    result.insert(b"u");
    result.insert(b"v");
    result.insert(b"w");
    result.insert(b"x");
    result.insert(b"y");
    result.insert(b"z");

    result
}

