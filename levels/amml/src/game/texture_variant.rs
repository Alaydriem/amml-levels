use amml_msvc::MsvcVariant;

use super::TexturePtrLayout;

/// The game's `std::variant<monostate, mce::TexturePtr, mce::ClientTexture, mce::ServerTexture>` (40 bytes: the
/// 32-byte TexturePtr storage, then the index byte), as `Mesh::renderMesh` takes it. A world mesh draw holds the
/// mod's white texture at `TexturePtrLayout::VARIANT_INDEX`: the name-tag material samples a texture, and the
/// empty `monostate` draws the missing-texture magenta
pub type TextureVariant = MsvcVariant<TexturePtrLayout>;

const _: () = assert!(size_of::<TextureVariant>() == 40);
