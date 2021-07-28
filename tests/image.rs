use std::path::PathBuf;

// verify that images in "examples/res/img" have the expected depth and channels.
#[test]
pub fn verify_res_img() {
    let path: PathBuf = "examples/res/img".into();

    macro_rules! assert_img {
        ($png:tt, $Variant:ident) => {
            let img = image::open(path.join($png)).unwrap();
            assert!(matches!(img, image::DynamicImage::$Variant(_)));
        };
    }

    assert_img!("Luma8.png", ImageLuma8);
    assert_img!("Luma16.png", ImageLuma16);
    assert_img!("LumaA8.png", ImageLumaA8);
    assert_img!("LumaA16.png", ImageLumaA16);
    assert_img!("RGB8.png", ImageRgb8);
    assert_img!("RGBA8.png", ImageRgba8);
    assert_img!("RGB16.png", ImageRgb16);
    assert_img!("RGBA16.png", ImageRgba16);
}
