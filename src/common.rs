#[derive(PartialEq, Eq, Hash)]
pub struct CrateVersion {
    name: String,
    version: String,
}

impl CrateVersion {
    pub fn new(name: &str, version: &str) -> Self {
        CrateVersion { name: name.to_string(), version: version.to_string() }
    }
}

// TODO: Would be nice to be able to check if a crate is in the hash table without creating a new instance of CrateVersion each time.
//       I don't think implementing Borrow is the right way to do this.
//       Is HashMap the best choice here?