{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
  	pkg-config
	rustc 
	cargo 
  ];

  # 2. Libraries needed at runtime / linked against
  buildInputs = with pkgs; [
  	alsa-lib
  ];
  PKG_CONFIG_PATH = "${pkgs.alsa-lib.dev}/lib/pkgconfig";

  shellHook = ''
  	echo "Rust development environment loaded!"
  '';
}
