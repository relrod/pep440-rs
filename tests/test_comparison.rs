use pep440::Version;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};

// Hasher that records the full sequence of 'write' calls, so can be used to make both
// equality and *inequality* assertions about hash implementations, without worrying
// about accidental collisions.
struct TestHasher {
    state: Vec<Vec<u8>>,
}

impl Hasher for TestHasher {
    fn finish(&self) -> u64 {
        return 0;
    }

    fn write(&mut self, bytes: &[u8]) {
        self.state.push(bytes.to_vec());
    }
}

fn hash_state<T: Hash>(obj: T) -> impl Eq {
    let mut hasher = TestHasher { state: vec![] };
    obj.hash(&mut hasher);
    hasher.state
}

#[test]
fn test_comparison() {
    let fh = File::open("tests/comparisons").expect("Could not open comparisons");
    let reader = BufReader::new(fh);

    for line in reader.lines() {
        let text = line.expect("Did not get a line");

        // Excuse the ugly parsing
        let split: Vec<&str> = text.split_whitespace().collect();
        let x = split.get(0).expect("Malformed input");
        let op = split.get(1).expect("Malformed input");
        let y = split.get(2).expect("Malformed input");

        let x_ver = Version::parse(x).expect(&format!("Could not parse LHS version: {}", x));
        let y_ver = Version::parse(y).expect(&format!("Could not parse RHS version: {}", y));

        match op {
            &">" => assert!(x_ver > y_ver, "Failed: {} > {}", x_ver, y_ver),
            &">=" => assert!(x_ver >= y_ver, "Failed: {} >= {}", x_ver, y_ver),
            &"<" => assert!(x_ver < y_ver, "Failed: {} < {}", x_ver, y_ver),
            &"<=" => assert!(x_ver <= y_ver, "Failed: {} <= {}", x_ver, y_ver),
            &"==" => {
                assert!(&x_ver == &y_ver, "Failed: {} == {}", x_ver, y_ver);
                assert!(
                    hash_state(&x_ver) == hash_state(&y_ver),
                    "Failed: hash(x) == hash(y)"
                );
            },
            &"!=" => {
                assert!(x_ver != y_ver, "Failed: {} != {}", x_ver, y_ver);
                assert!(
                    hash_state(&x_ver) != hash_state(&y_ver),
                    "Failed: hash(x) != hash(y)"
                );
            },
            _ => panic!("Operator did not make sense: {}", op),
        }
    }
}

#[test]
fn test_equality() {
    let cases: Vec<(&str, &str)> = vec![
        ("1.0", "1"),
        ("1.0.0.0.0", "1.0.0"),
        ("2.0.3a02.dev01", "2.0.3.0.0alpha2.dev1"),
        ("1+01", "1+1"),
        ("1+abc.123", "1+abc.000123"),
    ];

    for (a_str, b_str) in cases {
        println!("Checking {} and {}", a_str, b_str);
        let a_ver: Version = a_str.parse().expect("Malformed input");
        let b_ver: Version = b_str.parse().expect("Malformed input");
        assert!(a_ver == b_ver);
        assert!(a_ver >= b_ver);
        assert!(a_ver <= b_ver);
        assert!(!(a_ver != b_ver));
        assert!(!(a_ver < b_ver));
        assert!(!(a_ver > b_ver));
        assert!(hash_state(&a_ver) == hash_state(&b_ver));
    }
}
