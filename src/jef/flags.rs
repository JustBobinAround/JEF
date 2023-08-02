macro_rules! lock_as_mut {
    (|$var:ident | $custom_code: block) => {
        let $var = $var.clone();
        if let Ok(mut $var) = $var.lock(){
            $custom_code
        };
    };
}

macro_rules! lock_readonly {
    (|$var:ident | $custom_code: block) => {
        if let Ok($var) = $var.lock(){
            $custom_code
        };
    };
}

pub enum Flag {
    Nothing,
    Halt,
}

