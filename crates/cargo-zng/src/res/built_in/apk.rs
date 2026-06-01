use std::process::Command;

use super::*;

const APK_HELP: &str = r#"
Build an Android APK from a staging directory

The expected file system layout:

| apk/
| ├── lib/
| |   └── arm64-v8a
| |       └── my-app.so
| ├── assets/
| |   └── res
| |       └── zng-res.txt
| ├── res/
| |   └── android-res
| └── AndroidManifest.xml
| my-app.zr-apk

Both 'apk/' and 'my-app.zr-apk' will be replaced with the built my-app.apk

Expected .zr-apk file content:

| # Relative path to the staging directory. If not set uses ./apk if it exists
| # or the parent dir .. if it is named something.apk
| apk-dir = ./apk
|
| # Sign using the debug key. Note that if ZR_APK_KEYSTORE or ZR_APK_KEY_ALIAS are not
| # set the APK is also signed using the debug key.
| debug = true
|
| # Don't sign and don't zipalign the APK. This outputs an incomplete package that
| # cannot be installed, but can be modified such as custom linking and signing.
| raw = true
|
| # Don't tar assets. By default `assets/res` are packed as `assets/res.tar`
| # for use with `android_install_res`.
| tar-assets-res = false

APK signing is configured using these environment variables:

ZR_APK_KEYSTORE - path to the private .keystore file
ZR_APK_KEYSTORE_PASS - keystore file password
ZR_APK_KEY_ALIAS - key name in the keystore
ZR_APK_KEY_PASS - key password
"#;
pub(super) fn apk() {
    help(APK_HELP);
    if std::env::var(ZR_FINAL).is_err() {
        println!("zng-res::on-final=");
        return;
    }

    // read config
    let mut apk_dir = String::new();
    let mut debug = false;
    let mut raw = false;
    let mut tar_assets = true;
    for line in read_lines(&path(ZR_REQUEST)) {
        let (ln, line) = line.unwrap_or_else(|e| fatal!("error reading .zr-apk request, {e}"));
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            let bool_value = || match value {
                "true" => true,
                "false" => false,
                _ => {
                    error!("unexpected value, line {ln}\n   {line}");
                    false
                }
            };
            match key {
                "apk-dir" => apk_dir = value.to_owned(),
                "debug" => debug = bool_value(),
                "raw" => raw = bool_value(),
                "tar-assets" => tar_assets = bool_value(),
                _ => error!("unknown key, line {ln}\n   {line}"),
            }
        } else {
            error!("syntax error, line {ln}\n{line}");
        }
    }
    let mut keystore = PathBuf::from(env::var("ZR_APK_KEYSTORE").unwrap_or_default());
    let mut keystore_pass = env::var("ZR_APK_KEYSTORE_PASS").unwrap_or_default();
    let mut key_alias = env::var("ZR_APK_KEY_ALIAS").unwrap_or_default();
    let mut key_pass = env::var("ZR_APK_KEY_PASS").unwrap_or_default();
    if keystore.as_os_str().is_empty() || key_alias.is_empty() {
        debug = true;
    }

    let mut apk_folder = path(ZR_TARGET_DD);
    let output_file;
    if apk_dir.is_empty() {
        let apk = apk_folder.join("apk");
        if apk.exists() {
            apk_folder = apk;
            output_file = path(ZR_TARGET).with_extension("apk");
        } else if apk_folder.extension().map(|e| e.eq_ignore_ascii_case("apk")).unwrap_or(false) {
            output_file = apk_folder.clone();
        } else {
            fatal!("missing ./apk")
        }
    } else {
        apk_folder = apk_folder.join(apk_dir);
        if !apk_folder.is_dir() {
            fatal!("{} not found or not a directory", apk_folder.display());
        }
        output_file = path(ZR_TARGET).with_extension("apk");
    }
    let apk_folder = apk_folder;

    // find <sdk>/build-tools
    let android_home = match env::var("ANDROID_HOME") {
        Ok(h) if !h.is_empty() => h,
        _ => fatal!("please set ANDROID_HOME to the android-sdk dir"),
    };
    let build_tools = Path::new(&android_home).join("build-tools/");
    let mut best_build = None;
    let mut best_version = semver::Version::new(0, 0, 0);

    #[cfg(not(windows))]
    const AAPT2_NAME: &str = "aapt2";
    #[cfg(windows)]
    const AAPT2_NAME: &str = "aapt2.exe";

    for dir in fs::read_dir(build_tools).unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/build-tools/, {e}")) {
        let dir = dir
            .unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/build-tools/ entry, {e}"))
            .path();

        if let Some(ver) = dir
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| semver::Version::parse(f).ok())
            && ver > best_version
            && dir.join(AAPT2_NAME).exists()
        {
            best_build = Some(dir);
            best_version = ver;
        }
    }
    let build_tools = match best_build {
        Some(p) => p,
        None => fatal!("cannot find $ANDROID_HOME/build-tools/<version>/{AAPT2_NAME}"),
    };
    let aapt2_path = build_tools.join(AAPT2_NAME);

    // temp target dir
    let temp_dir = apk_folder.with_extension("apk.tmp");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir(&temp_dir).unwrap_or_else(|e| fatal!("cannot create {}, {e}", temp_dir.display()));

    // tar assets
    let assets = apk_folder.join("assets");
    let assets_res = assets.join("res");
    if tar_assets && assets_res.exists() {
        let tar_path = assets.join("res.tar");
        let r = Command::new("tar")
            .arg("-cf")
            .arg(&tar_path)
            .arg("res")
            .current_dir(&assets)
            .status();
        match r {
            Ok(s) => {
                if !s.success() {
                    fatal!("tar failed")
                }
            }
            Err(e) => fatal!("cannot run 'tar', {e}"),
        }
        if let Err(e) = fs::remove_dir_all(&assets_res) {
            fatal!("failed tar-assets-res cleanup, {e}")
        }
    }

    // build resources
    let compiled_res = temp_dir.join("compiled_res.zip");
    let res = apk_folder.join("res");
    if res.exists() {
        let mut aapt2 = Command::new(&aapt2_path);
        aapt2.arg("compile").arg("-o").arg(&compiled_res).arg("--dir").arg(res);

        if aapt2.status().map(|s| !s.success()).unwrap_or(true) {
            fatal!("resources build failed");
        }
    }

    let manifest_path = apk_folder.join("AndroidManifest.xml");
    let manifest = fs::read_to_string(&manifest_path).unwrap_or_else(|e| fatal!("cannot read AndroidManifest.xml, {e}"));
    let manifest: AndroidManifest = quick_xml::de::from_str(&manifest).unwrap_or_else(|e| fatal!("error parsing AndroidManifest.xml, {e}"));

    // find <sdk>/platforms
    let platforms = Path::new(&android_home).join("platforms");
    let mut best_platform = None;
    let mut best_version = 0;
    for dir in fs::read_dir(platforms).unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/platforms/, {e}")) {
        let dir = dir
            .unwrap_or_else(|e| fatal!("cannot read $ANDROID_HOME/platforms/ entry, {e}"))
            .path();

        if let Some(ver) = dir
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|f| f.strip_prefix("android-"))
            .and_then(|f| f.parse().ok())
            && manifest.uses_sdk.matches(ver)
            && ver > best_version
            && dir.join("android.jar").exists()
        {
            best_platform = Some(dir);
            best_version = ver;
        }
    }
    let platform = match best_platform {
        Some(p) => p,
        None => fatal!("cannot find $ANDROID_HOME/platforms/<version>/android.jar"),
    };

    // make apk (link)
    let apk_path = temp_dir.join("output.apk");
    let mut aapt2 = Command::new(&aapt2_path);
    aapt2
        .arg("link")
        .arg("-o")
        .arg(&apk_path)
        .arg("--manifest")
        .arg(manifest_path)
        .arg("-I")
        .arg(platform.join("android.jar"));
    if compiled_res.exists() {
        aapt2.arg(&compiled_res);
    }
    if assets.exists() {
        aapt2.arg("-A").arg(&assets);
    }
    if aapt2.status().map(|s| !s.success()).unwrap_or(true) {
        fatal!("apk linking failed");
    }

    // add libs
    let aapt_path = build_tools.join("aapt");
    for lib in ::glob::glob(apk_folder.join("lib/*/*.so").display().to_string().as_str()).unwrap() {
        let lib = lib.unwrap_or_else(|e| fatal!("error searching libs, {e}"));

        let lib = lib.display().to_string().replace('\\', "/");
        let lib = &lib[lib.rfind("/lib/").unwrap() + 1..];

        let mut aapt = Command::new(&aapt_path);
        aapt.arg("add").arg(&apk_path).arg(lib).current_dir(&apk_folder);
        if aapt.status().map(|s| !s.success()).unwrap_or(true) {
            fatal!("apk linking failed");
        }
    }

    let final_apk = if raw {
        apk_path
    } else {
        // align
        let aligned_apk_path = temp_dir.join("output-aligned.apk");
        let zipalign_path = build_tools.join("zipalign");
        let mut zipalign = Command::new(zipalign_path);
        zipalign.arg("-v").arg("4").arg(apk_path).arg(&aligned_apk_path);
        if zipalign.status().map(|s| !s.success()).unwrap_or(true) {
            fatal!("zipalign failed");
        }

        // sign
        let signed_apk_path = temp_dir.join("output-signed.apk");
        if debug {
            let dirs = directories::BaseDirs::new().unwrap_or_else(|| fatal!("cannot fine $HOME"));
            keystore = dirs.home_dir().join(".android/debug.keystore");
            keystore_pass = "android".to_owned();
            key_alias = "androiddebugkey".to_owned();
            key_pass = "android".to_owned();
            if !keystore.exists() {
                // generate debug.keystore
                let _ = fs::create_dir_all(keystore.parent().unwrap());
                let keytool_path = Path::new(&env::var("JAVA_HOME").expect("please set JAVA_HOME")).join("bin/keytool");
                let mut keytool = Command::new(&keytool_path);
                keytool
                    .arg("-genkey")
                    .arg("-v")
                    .arg("-keystore")
                    .arg(&keystore)
                    .arg("-storepass")
                    .arg(&keystore_pass)
                    .arg("-alias")
                    .arg(&key_alias)
                    .arg("-keypass")
                    .arg(&key_pass)
                    .arg("-keyalg")
                    .arg("RSA")
                    .arg("-keysize")
                    .arg("2048")
                    .arg("-validity")
                    .arg("10000")
                    .arg("-dname")
                    .arg("CN=Android Debug,O=Android,C=US")
                    .arg("-storetype")
                    .arg("pkcs12");

                match keytool.status() {
                    Ok(s) => {
                        if !s.success() {
                            fatal!("keytool failed generating debug keys");
                        }
                    }
                    Err(e) => fatal!("cannot run '{}', {e}", keytool_path.display()),
                }
            }
        }

        #[cfg(not(windows))]
        const APKSIGNER_NAME: &str = "apksigner";
        #[cfg(windows)]
        const APKSIGNER_NAME: &str = "apksigner.bat";

        let apksigner_path = build_tools.join(APKSIGNER_NAME);
        let mut apksigner = Command::new(&apksigner_path);
        apksigner
            .arg("sign")
            .arg("--ks")
            .arg(keystore)
            .arg("--ks-pass")
            .arg(format!("pass:{keystore_pass}"))
            .arg("--ks-key-alias")
            .arg(key_alias)
            .arg("--key-pass")
            .arg(format!("pass:{key_pass}"))
            .arg("--out")
            .arg(&signed_apk_path)
            .arg(&aligned_apk_path);

        match apksigner.status() {
            Ok(s) => {
                if !s.success() {
                    fatal!("apksigner failed")
                }
            }
            Err(e) => fatal!("cannot run '{}', {e}", apksigner_path.display()),
        }
        signed_apk_path
    };

    // finalize
    fs::remove_dir_all(&apk_folder).unwrap_or_else(|e| fatal!("apk folder cleanup failed, {e}"));
    fs::rename(final_apk, output_file).unwrap_or_else(|e| fatal!("cannot copy built apk to final place, {e}"));
    fs::remove_dir_all(&temp_dir).unwrap_or_else(|e| fatal!("temp dir cleanup failed, {e}"));
    let _ = fs::remove_file(path(ZR_TARGET));
}
#[derive(serde::Deserialize)]
#[serde(rename = "manifest")]
struct AndroidManifest {
    #[serde(rename = "uses-sdk")]
    #[serde(default)]
    pub uses_sdk: AndroidSdk,
}
#[derive(Default, serde::Deserialize)]
#[serde(rename = "uses-sdk")]
struct AndroidSdk {
    #[serde(rename(serialize = "android:minSdkVersion"))]
    pub min_sdk_version: Option<u32>,
    #[serde(rename(serialize = "android:targetSdkVersion"))]
    pub target_sdk_version: Option<u32>,
    #[serde(rename(serialize = "android:maxSdkVersion"))]
    pub max_sdk_version: Option<u32>,
}
impl AndroidSdk {
    pub fn matches(&self, version: u32) -> bool {
        if let Some(v) = self.target_sdk_version {
            return v == version;
        }
        if let Some(m) = self.min_sdk_version
            && version < m
        {
            return false;
        }
        if let Some(m) = self.max_sdk_version
            && version > m
        {
            return false;
        }
        true
    }
}
