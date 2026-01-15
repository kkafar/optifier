use optifier::{Partial, partial_derive};

#[derive(Partial, Debug)]
#[partial_derive(Debug)]
struct TestType {
    field_i32: i32,
    field_string: String,
    field_option_string: Option<String>,
}

fn main() -> Result<(), ()> {
    let _complete = TestType {
        field_i32: 42,
        field_string: String::from("Hello world"),
        field_option_string: Some(String::from("Hello world")),
    };
    let partial = TestTypePartial {
        field_i32: None,
        field_string: None,
        field_option_string: None,
    };
    let second_partial = TestTypePartial {
        field_i32: Some(42),
        field_string: None,
        field_option_string: Some(String::from("field_option_string")),
    };
    let merged = TestTypePartial::merge(partial, second_partial);

    dbg!(&merged);
    let config_result: Result<TestType, TestTypePartialError> = merged.try_into();

    let config = match config_result {
        Ok(config) => config,
        Err(err) => match err {
            TestTypePartialError::FieldI32Missing => {
                panic!("Field 'field_i32' is missing");
            }
            TestTypePartialError::FieldStringMissing => {
                panic!("Field 'field_string' is missing");
            }
        },
    };

    let _ = dbg!(config);

    return Ok(());
}
