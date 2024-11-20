use alloc::{collections::BTreeSet, format, string::String, sync::Arc, vec::Vec};
use core::fmt;

use assembly::{ast::QualifiedProcedureName, Library, Report};
use serde::{Deserialize, Serialize};
use vm_core::{mast::MastForest, utils::DisplayHex, Program};

use super::{de, se};
use crate::{Dependency, Digest};

// MAST ARTIFACT
// ================================================================================================

/// The artifact produced by lowering a [Program] to a Merkelized Abstract Syntax Tree
///
/// This type is used in compilation pipelines to abstract over the type of output requested.
#[derive(Debug, Clone)]
pub enum MastArtifact {
    /// A MAST artifact which can be executed by the VM directly
    Executable(Arc<Program>),
    /// A MAST artifact which can be used as a dependency by a [Program]
    Library(Arc<Library>),
}

impl MastArtifact {
    /// Get the underlying [Program] for this artifact, or panic if this is a [Library]
    pub fn unwrap_program(self) -> Arc<Program> {
        match self {
            Self::Executable(prog) => prog,
            Self::Library(_) => panic!("attempted to unwrap 'mast' library as program"),
        }
    }

    /// Get the underlying [Library] for this artifact, or panic if this is a [Program]
    pub fn unwrap_library(self) -> Arc<Library> {
        match self {
            Self::Executable(_) => panic!("attempted to unwrap 'mast' program as library"),
            Self::Library(lib) => lib,
        }
    }

    /// Get the content digest associated with this artifact
    pub fn digest(&self) -> Digest {
        match self {
            Self::Executable(ref prog) => prog.hash(),
            Self::Library(ref lib) => *lib.digest(),
        }
    }

    /// Get the underlying [MastForest] for this artifact
    pub fn mast_forest(&self) -> &MastForest {
        match self {
            Self::Executable(ref prog) => prog.mast_forest(),
            Self::Library(ref lib) => lib.mast_forest(),
        }
    }
}

// PACKAGE MANIFEST
// ================================================================================================

/// The manifest of a package, containing the set of package dependencies(libraries or packages) and
/// exported procedures and their signatures, if known.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PackageManifest {
    /// The set of exports in this package.
    pub exports: BTreeSet<PackageExport>,
    /// The libraries(packages) linked against by this package, which must be provided when
    /// executing the program.
    pub dependencies: Vec<Dependency>,
}

/// A procedure exported by a package, along with its digest and
/// signature(will be added after MASM type attributes are implemented).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub struct PackageExport {
    /// The fully-qualified name of the procedure exported by this package
    pub name: String,
    /// The digest of the procedure exported by this package
    #[serde(
        serialize_with = "se::serialize_digest",
        deserialize_with = "de::deserialize_digest"
    )]
    pub digest: Digest,
    // Signature will be added in the future when the type signatures are available in the Assembly
    // #[serde(default)]
    // pub signature: Option<Signature>,
}

impl fmt::Debug for PackageExport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageExport")
            .field("name", &format_args!("{}", self.name))
            .field("digest", &format_args!("{}", DisplayHex::new(&self.digest.as_bytes())))
            // .field("signature", &self.signature)
            .finish()
    }
}

// PACKAGE
// ================================================================================================

/// A package containing a [Program]/[Library], rodata segments,
/// and a manifest(exports and dependencies).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Name of the package
    pub name: String,
    /// The MAST artifact ([Program] or [Library]) of the package
    #[serde(serialize_with = "se::serialize_mast", deserialize_with = "de::deserialize_mast")]
    pub mast: MastArtifact,
    /// The package manifest, containing the set of exported procedures and their signatures,
    /// if known.
    pub manifest: PackageManifest,
}

impl Package {
    const MAGIC: &'static [u8] = b"MASP\0";
    const FORMAT_VERSION: &'static [u8] = b"1.0\0";

    /// Parses a package from the provided bytes
    pub fn read_from_bytes<B>(bytes: B) -> Result<Self, Report>
    where
        B: AsRef<[u8]>,
    {
        use alloc::borrow::Cow;

        let bytes = bytes.as_ref();

        let bytes = bytes
            .strip_prefix(Self::MAGIC)
            .ok_or_else(|| Report::msg("invalid package: missing header"))?;
        let bytes = bytes.strip_prefix(Self::FORMAT_VERSION).ok_or_else(|| {
            Report::msg(format!(
                "invalid package: incorrect version, expected '1.0', got '{}'",
                bytes.get(0..4).map(String::from_utf8_lossy).unwrap_or(Cow::Borrowed("")),
            ))
        })?;

        bitcode::deserialize(bytes).map_err(Report::msg)
    }

    /// Serializes the package into a byte array
    pub fn write_to_bytes(&self) -> Result<Vec<u8>, Report> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(Self::MAGIC);
        bytes.extend_from_slice(Self::FORMAT_VERSION);
        let mut data = bitcode::serialize(self).map_err(Report::msg)?;
        bytes.append(&mut data);
        Ok(bytes)
    }

    pub fn digest(&self) -> Digest {
        self.mast.digest()
    }

    /// Checks if the package's MAST artifact is a [Program]
    pub fn is_program(&self) -> bool {
        matches!(self.mast, MastArtifact::Executable(_))
    }

    /// Checks if the package's MAST artifact is a [Library]
    pub fn is_library(&self) -> bool {
        matches!(self.mast, MastArtifact::Library(_))
    }

    /// Unwraps the package's MAST artifact as a [Program] or panics if it is a [Library]
    pub fn unwrap_program(&self) -> Arc<Program> {
        match self.mast {
            MastArtifact::Executable(ref prog) => Arc::clone(prog),
            _ => panic!("expected package to contain a program, but got a library"),
        }
    }

    /// Unwraps the package's MAST artifact as a [Library] or panics if it is a [Program]
    pub fn unwrap_library(&self) -> Arc<Library> {
        match self.mast {
            MastArtifact::Library(ref lib) => Arc::clone(lib),
            _ => panic!("expected package to contain a library, but got an executable"),
        }
    }

    /// Creates a new package with [Program] from this [Library] package and the given
    /// entrypoint (should be a procedure in the library).
    pub fn make_executable(&self, entrypoint: &QualifiedProcedureName) -> Result<Self, Report> {
        let MastArtifact::Library(ref library) = self.mast else {
            return Err(Report::msg("expected library but got an executable"));
        };

        let module = library
            .module_infos()
            .find(|info| info.path() == &entrypoint.module)
            .ok_or_else(|| {
                Report::msg(format!(
                    "invalid entrypoint: library does not contain a module named '{}'",
                    entrypoint.module
                ))
            })?;
        if let Some(digest) = module.get_procedure_digest_by_name(&entrypoint.name) {
            let node_id = library.mast_forest().find_procedure_root(digest).ok_or_else(|| {
                Report::msg(
                    "invalid entrypoint: malformed library - procedure exported, but digest has \
                     no node in the forest",
                )
            })?;

            let exports = BTreeSet::from_iter(self.manifest.exports.iter().find_map(|export| {
                if export.digest == digest {
                    Some(export.clone())
                } else {
                    None
                }
            }));

            Ok(Self {
                name: self.name.clone(),
                mast: MastArtifact::Executable(Arc::new(Program::new(
                    library.mast_forest().clone(),
                    node_id,
                ))),
                manifest: PackageManifest {
                    exports,
                    dependencies: self.manifest.dependencies.clone(),
                },
            })
        } else {
            Err(Report::msg(format!(
                "invalid entrypoint: library does not export '{}'",
                entrypoint
            )))
        }
    }
}
