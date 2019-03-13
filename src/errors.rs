use error_chain::*;

error_chain! {

    errors {
        NotImplemented
    }

    foreign_links {
        Git(::git2::Error);
        Regex(::regex::Error);
    }
}
