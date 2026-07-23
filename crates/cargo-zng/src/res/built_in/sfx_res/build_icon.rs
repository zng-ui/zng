fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("{icon-path}");
        res.compile().unwrap();
    }
}
