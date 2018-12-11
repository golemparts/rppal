macro_rules! parse_retval {
  ($retval:expr) => {{
    let retval = $retval;

    if retval == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(retval)
    }
  }};
}
