// why a world mesh build could not run this frame; it is tried again next frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStall {
    // the render call gave no screen context
    NoScreen,
    // the screen context holds no tessellator
    NoTessellator,
    // a begin of another user is open on the tessellator
    TessellatorOpen,
    // the game heap is not reachable
    NoHeap,
    // `Tessellator::begin` returned without opening a begin
    BeginRefused,
    // `Tessellator::end` found no open begin after the fill
    NoMesh,
}

impl BuildStall {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoScreen => "no screen context",
            Self::NoTessellator => "no tessellator",
            Self::TessellatorOpen => "tessellator open",
            Self::NoHeap => "no game heap",
            Self::BeginRefused => "begin refused",
            Self::NoMesh => "end made no mesh",
        }
    }
}
