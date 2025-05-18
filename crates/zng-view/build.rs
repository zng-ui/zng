fn main() {
    cfg_aliases::cfg_aliases! {
        android: { target_os = "android" },
        ipc: { all(feature = "ipc", not(android)) },
    }

    tp_licenses();
}

fn tp_licenses() {
    #[cfg(feature = "bundle_licenses")]
    {
        let mut licenses = zng_tp_licenses::collect_cargo_about("../../.cargo/about.toml");

        avif_licenses(&mut licenses);

        zng_tp_licenses::write_bundle(&licenses);
    }
}

#[cfg(feature = "bundle_licenses")]
fn avif_licenses(l: &mut Vec<zng_tp_licenses::LicenseUsed>) {
    #[cfg(not(any(feature = "image_avif", zng_view_image_has_avif)))]
    let _ = l;
    #[cfg(any(feature = "image_avif", zng_view_image_has_avif))]
    {
        use zng_tp_licenses::*;

        l.push(LicenseUsed {
            license: License::new("BSD-2-Clause", r#"BSD 2-Clause "Simplified" License"#, DAV1D_COPYING),
            used_by: vec![User::new("dav1d", "1.3.0", "https://code.videolan.org/videolan/dav1d")],
        });

        const DAV1D_COPYING: &str = r##"
Copyright © 2018-2019, VideoLAN and dav1d authors
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

"##;
    }
}
