use std::fs;

pub fn load_file(path: Vec<String>) -> Result<String, std::io::Error> {
    let fpath = "examples/".to_string() + &path.join("/") + ".zg";
    println!("fpath: {}", fpath);
    let contents = fs::read_to_string(fpath)?;
    Ok(contents)
}
