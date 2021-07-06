//! # pep440
//!
//! This package provides a full Python
//! [PEP440](https://www.python.org/dev/peps/pep-0440/) parser for Rust.
//!
//! This crate, like the Python `packaging` test file from which many tests were
//! taken, is dual licensed under the terms of the Apache License, Version 2.0,
//! and the BSD License.
//!
//! The default mode uses a regex which is copied directly from the PEP440
//! specification, to do parsing. An alternative mode is planned for the future,
//! which will use the awesome [nom](https://github.com/Geal/nom)
//! parser-combinator library to do parsing. Both parsing modes will be
//! supported (once they are both implemented), and might have performance
//! differences, but should otherwise be identical.
//!
//! Currently, the following is implemented:
//!
//! * Parsing of version strings.
//! * An `is_canonical()` function which can check whether or not a version
//!   string is in canonical form.
//! * Tons of tests (copied from `packaging.version`).
#[macro_use]
extern crate lazy_static;

mod error;

use regex::{Captures, Regex};
use std::cmp::Ordering;
use std::hash::Hash;
use std::fmt;
use std::str::FromStr;

lazy_static! {
    /// A regex copied from bottom of PEP440 (notated by us) for determining
    /// whether or not a version is in canonical form.
    static ref CANONICAL_RE: Regex = Regex::new(r#"(?xi)
^([1-9][0-9]*!)?                                     # epoch
(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))*                  # release segment
((a|b|rc)(0|[1-9][0-9]*))?                           # pre-release
(\.post(0|[1-9][0-9]*))?                             # post release
(\.dev(0|[1-9][0-9]*))?                              # dev release
$"#).unwrap();

    /// A regex copied from the bottom of PEP440 and wrapped with `^` / `$`
    /// anchors, used for pulling out components of the given version number.
    static ref VERSION_RE: Regex = Regex::new(r#"(?xi)
^v?
(?:
    (?:(?P<epoch>[0-9]+)!)?                           # epoch
    (?P<release>[0-9]+(?:\.[0-9]+)*)                  # release segment
    (?P<pre>                                          # pre-release
        [-_\.]?
        (?P<pre_l>(a|b|c|rc|alpha|beta|pre|preview))
        [-_\.]?
        (?P<pre_n>[0-9]+)?
    )?
    (?P<post>                                         # post release
        (?:-(?P<post_n1>[0-9]+))
        |
        (?:
            [-_\.]?
            (?P<post_l>post|rev|r)
            [-_\.]?
            (?P<post_n2>[0-9]+)?
        )
    )?
    (?P<dev>                                          # dev release
        [-_\.]?
        (?P<dev_l>dev)
        [-_\.]?
        (?P<dev_n>[0-9]+)?
    )?
)
(?:\+(?P<local>[a-z0-9]+(?:[-_\.][a-z0-9]+)*))?       # local version
$"#).unwrap();
}

#[derive(Clone, Debug)]
/// Represents a version parsed as a PEP440-compliant version string.
///
/// Several things to note:
///
/// * All integer values are stored as `u32`. This is somewhat arbitrary, but
///   based on the fact that PEP440 defines no specification for these numbers,
///   beyond the fact that they are positive (thus our use of unsigned).
///
/// * The `release` component (i.e., the `1.2.3` of `v1.2.3rc0.post0.dev1+123`)
///   is stored as a vector of `u32` integers. This allows for easier ordering
///   comparison later.
///
/// * The `pre` component, if it exists, is stored as a `PreRelease`, which
///   allows for all of the valid pre-release identifiers that PEP440 specifies.
///   These are `a` (alpha), `b` (beta), and `rc` (release candidate).
///
/// * The `local` component is stored as a vector of `LocalVersion` components.
///   This is because the "local" version is allowed to contain both numeric and
///   string pieces, and we need to be able to account for both.
pub struct Version {
    pub epoch: u32,
    pub release: Vec<u32>,
    pub pre: Option<PreRelease>,
    pub post: Option<u32>,
    pub dev: Option<u32>,
    pub local: Vec<LocalVersion>,
}

impl Version {
    /// Returns `true` if the given version is in its canonical form, `false`
    /// if not.
    pub fn is_canonical(input: &str) -> bool {
        CANONICAL_RE.is_match(input)
    }

    /// Attempt to parse the given input string as a PEP440-compliant version
    /// string. By default, we base this on the same regex that is given at the
    /// bottom of the PEP440 specification. Release labels (`alpha`, `a`, `rc`,
    /// `dev`, `post`, etc.) are case-insensitive.
    pub fn parse(input: &str) -> Option<Version> {
        let captures = VERSION_RE.captures(input)?;

        fn pre_release_constructor(label: &str) -> Option<fn(u32) -> PreRelease> {
            match label {
                "a" => Some(PreRelease::A),
                "alpha" => Some(PreRelease::A),
                "b" => Some(PreRelease::B),
                "beta" => Some(PreRelease::B),
                "c" => Some(PreRelease::RC),
                "rc" => Some(PreRelease::RC),
                "pre" => Some(PreRelease::RC),
                "preview" => Some(PreRelease::RC),
                _ => None
            }
        }

        fn get_pre_release(captures: &Captures) -> Option<PreRelease> {
            let label = captures.name("pre_l").map(|pre_l| pre_l.as_str())?;
            let constructor = pre_release_constructor(&label.to_lowercase())?;
            let pre_n = captures
                .name("pre_n")
                .and_then(|pre_n| pre_n.as_str().parse().ok())
                .unwrap_or(0);
            Some(constructor(pre_n))
        }

        fn get_post_release(captures: &Captures) -> Option<u32> {
            match captures.name("post_n1") {
                Some(n1) => n1.as_str().parse().ok(),
                None => match captures.name("post_n2") {
                    Some(n2) => n2.as_str().parse().ok(),
                    None => match captures.name("post_l") {
                        // "1.2.3.post" -- default 0 in this case
                        Some(_) => Some(0),
                        None => None
                    }
                }
            }
        }

        fn get_dev_release(captures: &Captures) -> Option<u32> {
            captures.name("dev_l")?; // Bail out if no label
            match captures.name("dev_n") {
                Some(dev_n) => dev_n.as_str().parse().ok(),
                None => Some(0),
            }
        }

        fn get_local_component(component: &str) -> LocalVersion {
            if let Ok(num) = component.parse::<u32>() {
                LocalVersion::NumericComponent(num)
            } else {
                LocalVersion::StringComponent(component.to_lowercase())
            }
        }

        let epoch: u32 = captures
            .name("epoch")
            .map(|epoch| epoch.as_str())
            .unwrap_or("0")
            .parse()
            .ok()?;
        let release = captures
            .name("release")?
            .as_str()
            .split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<Vec<u32>>>()?;
        let pre = get_pre_release(&captures);
        let post = get_post_release(&captures);
        let dev = get_dev_release(&captures);
        let local: Vec<LocalVersion> = captures
            .name("local")
            .map(|local| local.as_str().split(&['-', '_', '.'][..]).collect::<Vec<&str>>())
            .unwrap_or_default()
            .iter()
            .map(|local| get_local_component(local))
            .collect();
        Some(Version { epoch, release, pre, post, dev, local })
    }

    /// Returns the normalized form of the epoch for the version.
    /// This will either be a number followed by a `!`, or the empty string.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("1!2.3.4rc0").unwrap();
    /// assert_eq!(ver.epoch_str(), "1!".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4rc0").unwrap();
    /// assert_eq!(ver.epoch_str(), "".to_string());
    /// ```
    pub fn epoch_str(&self) -> String {
        if self.epoch != 0 {
            format!("{}!", self.epoch)
        } else {
            "".to_string()
        }
    }

    /// Returns the normalized form of the release for the version.
    /// This will be the release component of the input, but with leading zeros
    /// removed from each segment.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4post3.dev6").unwrap();
    /// assert_eq!(ver.release_str(), "2.3.4".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("v002.03.000004post3.dev6").unwrap();
    /// assert_eq!(ver.release_str(), "2.3.4".to_string());
    /// ```
    pub fn release_str(&self) -> String {
        self.release
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(".")
    }

    /// Returns the normalized form of the pre-release field for the version.
    /// If no pre-release is given, the empty string will be returned.
    ///
    /// Non-canonical strings will be turned into canonical ones. For example,
    /// `alpha3` will be turned into `a3`, and `preview9` will be turned into
    /// `rc9`.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4c4.post3.dev6").unwrap();
    /// assert_eq!(ver.pre_str(), "rc4".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4.alpha8").unwrap();
    /// assert_eq!(ver.pre_str(), "a8".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4").unwrap();
    /// assert_eq!(ver.pre_str(), "".to_string());
    /// ```
    pub fn pre_str(&self) -> String {
        self.pre
            .clone() // ?
            .map(|x| format!("{}", x))
            .unwrap_or_default()
    }

    /// Returns the normalized form of the post-release field for the version.
    /// If no post-release is given, the empty string will be returned.
    ///
    /// If a string is returned, it includes a leading `.` which is required in
    /// normalized renditions of a version.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4c4.post3.dev6").unwrap();
    /// assert_eq!(ver.post_str(), ".post3".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4-3.dev6").unwrap();
    /// assert_eq!(ver.post_str(), ".post3".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4.alpha8").unwrap();
    /// assert_eq!(ver.post_str(), "".to_string());
    /// ```
    pub fn post_str(&self) -> String {
        self.post
            .map(|x| format!(".post{}", x))
            .unwrap_or_default()
    }

    /// Returns the normalized form of the dev-release field for the version.
    /// If no dev-release is given, the empty string will be returned.
    ///
    /// If a string is returned, it includes a leading `.` which is required in
    /// normalized renditions of a version.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4c4.post3.dev6").unwrap();
    /// assert_eq!(ver.dev_str(), ".dev6".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4.alpha8").unwrap();
    /// assert_eq!(ver.dev_str(), "".to_string());
    /// ```
    pub fn dev_str(&self) -> String {
        self.dev
            .map(|x| format!(".dev{}", x))
            .unwrap_or_default()
    }

    /// Returns the normalized form of the local field for the version.
    /// If no local component is given, the empty string will be returned.
    ///
    /// If a string is returned, it includes a leading `+` which is required in
    /// normalized renditions of a version.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4c4.post3.dev6+123.foo_deb-3").unwrap();
    /// assert_eq!(ver.local_str(), "+123.foo.deb.3".to_string());
    /// ```
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4.alpha8").unwrap();
    /// assert_eq!(ver.local_str(), "".to_string());
    /// ```
    pub fn local_str(&self) -> String {
        let glued = self.local
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join(".");
        if !glued.is_empty() {
            format!("+{}", glued)
        } else {
            "".to_string()
        }
    }

    /// Returns public portion of the version in normalized form.
    ///
    /// This is equivalent to all components except the "local" portion of the
    /// version.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("2.3.4c4.post3.dev6+123.foo_deb-3").unwrap();
    /// assert_eq!(ver.public_str(), "2.3.4rc4.post3.dev6".to_string());
    /// ```
    pub fn public_str(&self) -> String {
        format!(
            "{}{}{}{}{}",
            self.epoch_str(),
            self.release_str(),
            self.pre_str(),
            self.post_str(),
            self.dev_str())
    }

    /// Returns the normalized form of the version by combining all of the
    /// segments in their normalized form as returned by the `*_str()` methods
    /// defined above.
    ///
    /// This method is also used to implement `Display` for `Version` and the
    /// result will be identical.
    ///
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("v2.3.4c4.post3.dev6+1.f-3").unwrap();
    /// assert_eq!(ver.normalize(), "2.3.4rc4.post3.dev6+1.f.3".to_string());
    /// ```
    pub fn normalize(&self) -> String {
        format!("{}{}", self.public_str(), self.local_str())
    }
}

/// This implementation is returns the normalized version of the version.
/// It is equivalent to calling `normalize()` on the version.
impl fmt::Display for Version {
    /// ```
    /// # use pep440::Version;
    /// let ver = Version::parse("v2.3.4c4.post3.dev6+1.f-3").unwrap();
    /// assert_eq!(format!("{}", ver), ver.normalize());
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.normalize())
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    /// ```
    /// # use pep440::Version;
    /// # use std::cmp::Ordering;
    /// let ver1 = Version::parse("v2.3.4c4.post3.dev6+1.f-3").unwrap();
    /// let ver2 = Version::parse("v2.3.4pre4.post3.dev6+1.f-3").unwrap();
    /// assert_eq!(ver1.partial_cmp(&ver2), Some(Ordering::Equal))
    /// ```
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Modelled after packaging.version._cmpkey
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CmpKey<'a> {
    epoch: u32,
    trimmed_release: &'a [u32],
    // possible values:
    //   (0, 0) -> like version.py's NegativeInfinity
    //   (1, ...) -> alpha
    //   (2, ...) -> beta
    //   (3, ...) -> rc
    //   (4, 0) -> like version.py's Infinity
    pre: (u32, u32),
    // No-post is smaller than .post0, so we map:
    //   None -> 0, Some(n) -> n+1
    // which requires a wider type.
    post: u64,
    // Widen so we can map no-dev to something larger than u32::MAX
    dev: u64,
    local: &'a [LocalVersion],
}

impl Version {
    fn cmp_key(&self) -> CmpKey {
        let epoch = self.epoch;

        // We want to ignore trailing zeros in the release, so make a slice of just the
        // initial part of the release vector.
        let mut trimmed_release = &self.release[..];
        while trimmed_release.ends_with(&[0]) {
            trimmed_release = &trimmed_release[..trimmed_release.len() - 1];
        }

        // Quoting from packaging/version.py:
        // We need to "trick" the sorting algorithm to put 1.0.dev0 before 1.0a0.
        // We'll do this by abusing the pre segment, but we _only_ want to do this
        // if there is not a pre or a post segment. If we have one of those then
        // the normal sorting rules will handle this case correctly.
        let pre = if self.pre.is_none() && self.post.is_none() && self.dev.is_some() {
            (0, 0)  // = version.py's NegativeInfinity
        } else {
            match self.pre {
                None => (4, 0),  // = version.py's Infinity
                Some(PreRelease::A(n)) => (1, n),
                Some(PreRelease::B(n)) => (2, n),
                Some(PreRelease::RC(n)) => (3, n),
            }
        };

        let post = match self.post {
            None => 0,
            Some(n) => (n + 1).into(),
        };

        let dev = match self.dev {
            Some(n) => n.into(),
            None => u64::MAX,
        };

        let local = &self.local;

        CmpKey {
            epoch,
            trimmed_release,
            pre,
            post,
            dev,
            local,
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_key().cmp(&other.cmp_key())
    }
}

impl Hash for Version {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cmp_key().hash(state);
    }
}

impl FromStr for Version {
    type Err = error::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Version::parse(s) {
            Some(v) => Ok(v),
            _ => Err(error::Error::parse_error(s.to_string())),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
/// Segments of the "local" part of a version (anything after a `+`).
///
/// These segments can either be strings or numbers, and we store them in a
/// vector in `Version`, so we need to be able to store both.
///
/// Order-comparison of the segments also works differently dependending on
/// whether or not the segment is purely numeric.
pub enum LocalVersion {
    NumericComponent(u32),
    StringComponent(String),
}

impl fmt::Display for LocalVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LocalVersion::*;
        match self {
            NumericComponent(n) => write!(f, "{}", n),
            StringComponent(s) => write!(f, "{}", s),
        }
    }
}

impl PartialOrd for LocalVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LocalVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        use LocalVersion::*;
        match (self, other) {
            (NumericComponent(n1), NumericComponent(n2)) => n1.cmp(n2),
            (StringComponent(s1), StringComponent(s2)) => s1.cmp(s2),
            (NumericComponent(_), StringComponent(_)) => Ordering::Greater,
            (StringComponent(_), NumericComponent(_)) => Ordering::Less,
        }
    }
}


#[derive(Clone, Eq, PartialEq, Debug)]
/// The pre-release component of a version, such as `rcN`, `bN`, or `aN`.
///
/// We don't implement `Ord` on `PreRelease` because the context (`Versions`)
/// under consideration matter. In other words, `1.2.3b1 < 1.2.3rc1`, but
/// `1.2.4b1 > 1.2.3rc1`. If we allowed for comparing `PreReleases` alone, in
/// the first scenario, we would have `B(1) < RC(1)` and in the second scenario
/// we would have `B(1) > RC(1)`. So we only implement comparison of
/// `PreRelease` as part of the definition of comparison of `Version` as a
/// whole.
pub enum PreRelease {
    /// Release Candidate
    RC(u32),
    /// Alpha
    A(u32),
    /// Beta
    B(u32),
}

impl fmt::Display for PreRelease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreRelease::RC(v) => write!(f, "rc{}", v),
            PreRelease::A(v) => write!(f, "a{}", v),
            PreRelease::B(v) => write!(f, "b{}", v),
        }
    }
}

#[cfg(test)]
/// Much of the test data here is pulled directly from the python-packaging
/// tests, https://github.com/pypa/packaging/blob/main/tests/test_version.py
mod tests {
    use crate::*;

    const CANONICAL_VERSIONS: &[&str] = &[
        "1!1.0", "1!1.0.dev456", "1!1.0.post456",
        "1!1.0.post456.dev34", "1!1.0a1", "1!1.0a12",
        "1!1.0a12.dev456", "1!1.0a2.dev456", "1!1.0b1.dev456",
        "1!1.0b2", "1!1.0b2.post345", "1!1.0b2.post345.dev456",
        "1!1.0b2.post346", "1!1.0rc1", "1!1.0rc1.dev456", "1!1.0rc2",
        "1!1.0rc3", "1!1.1.dev1", "1.0", "1.0.dev456", "1.0.post456",
        "1.0.post456.dev34", "1.0a1", "1.0a12", "1.0a12.dev456",
        "1.0a2.dev456", "1.0b1.dev456", "1.0b2", "1.0b2.post345",
        "1.0b2.post345.dev456", "1.0b2.post346", "1.0rc1",
        "1.0rc1.dev456", "1.0rc2", "1.0rc3", "1.1.dev1",
    ];

    const NON_CANONICAL_VERSIONS: &[&str] = &[
        "1!1.2+1234.abc", "1!1.2+123456", "1!1.2+123abc", "1!1.2+123abc456",
        "1!1.2+abc", "1!1.2+abc123", "1!1.2+abc123def", "1!1.2.post32+123456",
        "1!1.2.post33+123456", "1.2+1234.abc", "1.2+123456", "1.2+123abc",
        "1.2+123abc456", "1.2+abc", "1.2+abc123", "1.2+abc123def",
        "1.2.post32+123456", "1.2.post33+123456", "1!1.2.post32+12_34.56",
        "1!1.2.post32+12.oh.no.whyyyyy.56",
    ];

    const INVALID_VERSIONS: &[&str] = &[
        "foo bar", "1.0+a+", "1.0++", "1.0+_foobar", "1.0+foo&asd", "1.0+1+1",
        "-3.0", "version 4", "beta alpha", "4.0ba3", "seven", "7..34", "1.0bb3",
        "1.0omega4",
    ];

    const NORMALIZATION: &[(&str, &str)] = &[
        ("1.0dev", "1.0.dev0"), ("1.0.dev", "1.0.dev0"),
        ("1.0dev1", "1.0.dev1"), ("1.0dev", "1.0.dev0"),
        ("1.0-dev", "1.0.dev0"), ("1.0-dev1", "1.0.dev1"),
        ("1.0DEV", "1.0.dev0"), ("1.0.DEV", "1.0.dev0"),
        ("1.0DEV1", "1.0.dev1"), ("1.0DEV", "1.0.dev0"),
        ("1.0.DEV1", "1.0.dev1"), ("1.0-DEV", "1.0.dev0"),
        ("1.0-DEV1", "1.0.dev1"), ("1.0a", "1.0a0"), ("1.0.a", "1.0a0"),
        ("1.0.a1", "1.0a1"), ("1.0-a", "1.0a0"), ("1.0-a1", "1.0a1"),
        ("1.0alpha", "1.0a0"), ("1.0.alpha", "1.0a0"), ("1.0.alpha1", "1.0a1"),
        ("1.0-alpha", "1.0a0"), ("1.0-alpha1", "1.0a1"), ("1.0A", "1.0a0"),
        ("1.0.A", "1.0a0"), ("1.0.A1", "1.0a1"), ("1.0-A", "1.0a0"),
        ("1.0-A1", "1.0a1"), ("1.0ALPHA", "1.0a0"), ("1.0.ALPHA", "1.0a0"),
        ("1.0.ALPHA1", "1.0a1"), ("1.0-ALPHA", "1.0a0"),
        ("1.0-ALPHA1", "1.0a1"), ("1.0b", "1.0b0"), ("1.0.b", "1.0b0"),
        ("1.0.b1", "1.0b1"), ("1.0-b", "1.0b0"), ("1.0-b1", "1.0b1"),
        ("1.0beta", "1.0b0"), ("1.0.beta", "1.0b0"), ("1.0.beta1", "1.0b1"),
        ("1.0-beta", "1.0b0"), ("1.0-beta1", "1.0b1"), ("1.0B", "1.0b0"),
        ("1.0.B", "1.0b0"), ("1.0.B1", "1.0b1"), ("1.0-B", "1.0b0"),
        ("1.0-B1", "1.0b1"), ("1.0BETA", "1.0b0"), ("1.0.BETA", "1.0b0"),
        ("1.0.BETA1", "1.0b1"), ("1.0-BETA", "1.0b0"), ("1.0-BETA1", "1.0b1"),
        ("1.0c", "1.0rc0"), ("1.0.c", "1.0rc0"), ("1.0.c1", "1.0rc1"),
        ("1.0-c", "1.0rc0"), ("1.0-c1", "1.0rc1"), ("1.0rc", "1.0rc0"),
        ("1.0.rc", "1.0rc0"), ("1.0.rc1", "1.0rc1"), ("1.0-rc", "1.0rc0"),
        ("1.0-rc1", "1.0rc1"), ("1.0C", "1.0rc0"), ("1.0.C", "1.0rc0"),
        ("1.0.C1", "1.0rc1"), ("1.0-C", "1.0rc0"), ("1.0-C1", "1.0rc1"),
        ("1.0RC", "1.0rc0"), ("1.0.RC", "1.0rc0"), ("1.0.RC1", "1.0rc1"),
        ("1.0-RC", "1.0rc0"), ("1.0-RC1", "1.0rc1"), ("1.0post", "1.0.post0"),
        ("1.0.post", "1.0.post0"), ("1.0post1", "1.0.post1"),
        ("1.0post", "1.0.post0"), ("1.0-post", "1.0.post0"),
        ("1.0-post1", "1.0.post1"), ("1.0POST", "1.0.post0"),
        ("1.0.POST", "1.0.post0"), ("1.0POST1", "1.0.post1"),
        ("1.0POST", "1.0.post0"), ("1.0r", "1.0.post0"),
        ("1.0rev", "1.0.post0"), ("1.0.POST1", "1.0.post1"),
        ("1.0.r1", "1.0.post1"), ("1.0.rev1", "1.0.post1"),
        ("1.0-POST", "1.0.post0"), ("1.0-POST1", "1.0.post1"),
        ("1.0-5", "1.0.post5"), ("1.0-r5", "1.0.post5"),
        ("1.0-rev5", "1.0.post5"), ("1.0+AbC", "1.0+abc"), ("1.01", "1.1"),
        ("1.0a05", "1.0a5"), ("1.0b07", "1.0b7"), ("1.0c056", "1.0rc56"),
        ("1.0rc09", "1.0rc9"), ("1.0.post000", "1.0.post0"),
        ("1.1.dev09000", "1.1.dev9000"), ("00!1.2", "1.2"),
        ("0100!0.0", "100!0.0"), ("v1.0", "1.0"),
        ("1.0.dev456", "1.0.dev456"), ("1.0a1", "1.0a1"),
        ("1.0a2.dev456", "1.0a2.dev456"), ("1.0a12.dev456", "1.0a12.dev456"),
        ("1.0a12", "1.0a12"), ("1.0b1.dev456", "1.0b1.dev456"),
        ("1.0b2", "1.0b2"), ("1.0b2.post345.dev456", "1.0b2.post345.dev456"),
        ("1.0b2.post345", "1.0b2.post345"), ("1.0rc1.dev456", "1.0rc1.dev456"),
        ("1.0rc1", "1.0rc1"), ("1.0.post456.dev34", "1.0.post456.dev34"),
        ("1.0.post456", "1.0.post456"), ("1.0.1", "1.0.1"),
        ("0!1.0.2", "1.0.2"), ("1.0.3+7", "1.0.3+7"),
        ("0!1.0.4+8.0", "1.0.4+8.0"), ("1.0.5+9.5", "1.0.5+9.5"),
        ("1.2+1234.abc", "1.2+1234.abc"), ("1.2+123456", "1.2+123456"),
        ("1.2+123abc", "1.2+123abc"), ("1.2+123abc456", "1.2+123abc456"),
        ("1.2+abc", "1.2+abc"), ("1.2+abc123", "1.2+abc123"),
        ("1.2+abc123def", "1.2+abc123def"), ("1.1.dev1", "1.1.dev1"),
        ("7!1.0.dev456", "7!1.0.dev456"), ("7!1.0a1", "7!1.0a1"),
        ("7!1.0a2.dev456", "7!1.0a2.dev456"),
        ("7!1.0a12.dev456", "7!1.0a12.dev456"), ("7!1.0a12", "7!1.0a12"),
        ("7!1.0b1.dev456", "7!1.0b1.dev456"), ("7!1.0b2", "7!1.0b2"),
        ("7!1.0b2.post345.dev456", "7!1.0b2.post345.dev456"),
        ("7!1.0b2.post345", "7!1.0b2.post345"),
        ("7!1.0rc1.dev456", "7!1.0rc1.dev456"), ("7!1.0rc1", "7!1.0rc1"),
        ("7!1.0", "7!1.0"), ("7!1.0.post456.dev34", "7!1.0.post456.dev34"),
        ("7!1.0.post456", "7!1.0.post456"), ("7!1.0.1", "7!1.0.1"),
        ("7!1.0.2", "7!1.0.2"), ("7!1.0.3+7", "7!1.0.3+7"),
        ("7!1.0.4+8.0", "7!1.0.4+8.0"), ("7!1.0.5+9.5", "7!1.0.5+9.5"),
        ("7!1.1.dev1", "7!1.1.dev1"),
    ];

    #[test]
    fn test_is_canonical() {
        for version in CANONICAL_VERSIONS {
            assert!(
                Version::is_canonical(version),
                "Expected '{}' to be valid", version);
        }

        for version in NON_CANONICAL_VERSIONS {
            assert!(
                !Version::is_canonical(version),
                "Expected '{}' to NOT be valid", version);
        }
    }

    #[test]
    fn test_parse() {
        for version in [CANONICAL_VERSIONS, NON_CANONICAL_VERSIONS].concat() {
            assert!(
                Version::parse(version).is_some(),
                "Failed to parse version: '{}'", version);
        }

        for version in INVALID_VERSIONS {
            assert!(
                Version::parse(version).is_none(),
                "Parsed version but should not have: '{}'", version);
        }
    }

    #[test]
    fn test_fromstr() {
        for version in [CANONICAL_VERSIONS, NON_CANONICAL_VERSIONS].concat() {
            assert!(
                Version::from_str(version).is_ok(),
                "Failed to parse version: '{}'", version);
        }

        for version in INVALID_VERSIONS {
            let invalid = Version::from_str(version);
            assert!(
                invalid.is_err(),
                "Parsed version but should not have: '{}'", version);
            assert_eq!(
                format!("{}", invalid.unwrap_err()),
                format!("Failed to parse version: {}", version));
        }
    }

    #[test]
    fn test_normalization() {
        for (input, expected) in NORMALIZATION {
            let ver = Version::parse(input);
            assert!(ver.is_some(), "Failed to parse version: {}", input);
            let normalized = ver.unwrap().normalize();
            let expected = expected.to_string();
            assert_eq!(
                normalized,
                expected,
                "input={}, expected={}, actual={}",
                input,
                expected,
                normalized);
        }
    }

    // Comparison testing is done in tests/* due to use of an external file.
}
