fn main() {
    #[cfg(target_os = "windows")]
    {
        if let Err(error) = embed_windows_resources() {
            panic!("failed to embed Windows resources: {}", error);
        }
    }
}

#[cfg(target_os = "windows")]
fn embed_windows_resources() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let png_icon_path = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .ok_or("failed to resolve workspace root")?
        .join("assets")
        .join("icon.png");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let ico_icon_path = out_dir.join("fcr-reminder.ico");

    let image = image::io::Reader::open(&png_icon_path)?
        .with_guessed_format()?
        .decode()?;
    let rgba = image.to_rgba8();
    let icon_image = ico::IconImage::from_rgba_data(image.width(), image.height(), rgba.into_raw());
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image)?);

    let icon_file = File::create(&ico_icon_path)?;
    icon_dir.write(icon_file)?;

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(ico_icon_path.to_string_lossy().as_ref());
    resource.set("ProductName", "FCR Reminder");
    resource.set("FileDescription", "FCR Reminder");
    resource.set("InternalName", "fcr-reminder");
    resource.set("OriginalFilename", "fcr-reminder.exe");
    resource.set("CompanyName", "Full Calendar Remastered");
    resource.set("LegalCopyright", "Copyright (c) Full Calendar Remastered");
    resource.compile()?;

    println!("cargo:rerun-if-changed={}", png_icon_path.display());
    Ok(())
}