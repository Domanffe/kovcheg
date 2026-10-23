{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  nodejs_22,
  wl-clipboard,
  xclip,
}:

buildNpmPackage (finalAttrs: {
  pname = "kovcheg";
  version = "1.5.0";
  __structuredAttrs = true;
  strictDeps = true;

  src = fetchFromGitHub {
    owner = "Domanffe";
    repo = "kovcheg-gui";
    tag = "v${finalAttrs.version}";
    # Update after release: nix-prefetch-url --unpack https://github.com/Domanffe/kovcheg/archive/v1.5.0.tar.gz
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  nodejs = nodejs_22;
  # Update after changing package-lock.json: nix-build -A npmDeps
  npmDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

  npmFlags = [ "--ignore-scripts" ];

  postBuild = ''
    node scripts/postbuild.cjs
  '';

  postInstall = ''
    wrapProgram $out/bin/kovcheg \
      --prefix PATH : ${lib.makeBinPath [ wl-clipboard xclip ]}
  '';

  meta = {
    description = "Kovcheg browser search CLI (desktop downloads use the Tauri app)";
    homepage = "https://github.com/Domanffe/kovcheg";
    changelog = "https://github.com/Domanffe/kovcheg/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mit;
    mainProgram = "kovcheg";
    platforms = lib.platforms.linux;
  };
})
