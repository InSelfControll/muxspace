{
  description = "Muxspace development environment";

  inputs.nixpkgs.url = "nixpkgs";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        GIO_EXTRA_MODULES = "${pkgs.glib-networking}/lib/gio/modules";
        GST_PLUGIN_SYSTEM_PATH_1_0 = pkgs.lib.makeSearchPath "lib/gstreamer-1.0" [
          pkgs.gst_all_1.gstreamer
          pkgs.gst_all_1.gst-plugins-base
          pkgs.gst_all_1.gst-plugins-good
          pkgs.gst_all_1.gst-plugins-bad
          pkgs.gst_all_1.gst-libav
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustc
          cargo
          clippy
          rustfmt
        ];

        buildInputs = with pkgs; [
          glib
          glib-networking
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-bad
          gst_all_1.gst-libav
          gtk3
          webkitgtk_4_1
          openssl
          xdotool
        ];
      };
    };
}
