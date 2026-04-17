/// A block flavour identifier — a `&'static str` tag that uniquely names a block type.
pub type Flavour = &'static str;

pub const PROSE: Flavour = "nom:prose";
pub const NOMX: Flavour = "nom:nomx";
pub const MEDIA_IMAGE: Flavour = "nom:media-image";
pub const MEDIA_ATTACHMENT: Flavour = "nom:media-attachment";
pub const GRAPH_NODE: Flavour = "nom:graph-node";
pub const DRAWING: Flavour = "nom:drawing";
pub const TABLE: Flavour = "nom:table";
pub const EMBED: Flavour = "nom:embed";
pub const NOTE: Flavour = "nom:note";
pub const SURFACE: Flavour = "nom:surface";
pub const CALLOUT: Flavour = "nom:callout";
