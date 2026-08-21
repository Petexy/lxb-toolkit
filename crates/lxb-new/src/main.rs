fn main() {
    if let Err(err) = lxb_new::run(std::env::args_os().skip(1)) {
        eprintln!("lxb-new: {err}");
        std::process::exit(2);
    }
}
