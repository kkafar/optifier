use optifier;

#[derive(optifier::Partial)]
#[derive(Debug)]
struct TestType {
    field_i32: i32,
    field_string: String,
    field_option_string: Option<String>,
}

fn main() -> Result<(), ()> {
    let _complete = TestType { field_i32: 42, field_string: String::from("Hello world"), field_option_string: Some(String::from("Hello world")) };
    let partial = TestTypePartial { field_i32: Some(42), field_string: Some(String::from("hello world")), field_option_string: Some(String::from("hello world")) };
    dbg!(partial);
    return Ok(());
}
