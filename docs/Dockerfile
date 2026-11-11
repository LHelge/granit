# The documentation site, https://granit.lhelge.se, as an nginx image. Built and pushed by the
# `docs` job of .github/workflows/release.yml after aphid has rendered the site into docs/dist/,
# then deployed to hoster (stack file: docs/docs.stack.toml).
#
# nginx-unprivileged runs as non-root and listens on 8080, which suits rootless podman.
FROM docker.io/nginxinc/nginx-unprivileged:stable-alpine

COPY default.conf /etc/nginx/conf.d/default.conf
COPY dist/ /usr/share/nginx/html/
