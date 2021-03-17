use pep440::Version;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[test]
fn test_comparison() {
    let fh = File::open("tests/comparisons")
        .expect("Could not open comparisons");
    let reader = BufReader::new(fh);

    for line in reader.lines() {
        let text = line.expect("Did not get a line");

        // Excuse the ugly parsing
        let split: Vec<&str> = text.split_whitespace().collect();
        let x = split.get(0).expect("Malformed input");
        let op = split.get(1).expect("Malformed input");
        let y = split.get(2).expect("Malformed input");

        let x_ver = Version::parse(x).expect(
            &format!("Could not parse LHS version: {}", x));
        let y_ver = Version::parse(y).expect(
            &format!("Could not parse RHS version: {}", y));

        match op {
            &">" => assert!(x_ver > y_ver, "Failed: {} > {}", x_ver, y_ver),
            &">=" => assert!(x_ver >= y_ver, "Failed: {} >= {}", x_ver, y_ver),
            &"<" => assert!(x_ver < y_ver, "Failed: {} < {}", x_ver, y_ver),
            &"<=" => assert!(x_ver <= y_ver, "Failed: {} <= {}", x_ver, y_ver),
            &"==" => assert!(x_ver == y_ver, "Failed: {} == {}", x_ver, y_ver),
            &"!=" => assert!(x_ver != y_ver, "Failed: {} != {}", x_ver, y_ver),
            _ => panic!(format!("Operator did not make sense: {}", op)),
        }
    }
}
