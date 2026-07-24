use std::fs::File;
use std::io::Read;

pub fn load_file(path: Vec<String>) -> Result<String, std::io::Error> {
    let fpath = path.join("/") + ".zg";
    let mut file = File::open(fpath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
