{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    rustup
    openssl
    pkg-config
    gcc
    kubectl
    helm
    k9s
  ];
}