# Cloudflare edge headers

These are applied manually in the Cloudflare dashboard for `paul-maxwell.com`
— the repo has no Cloudflare API access and doesn't push these. This
document is the paste-ready reference; `security/csp-hashes.txt` is the
source of truth for the two hash values below.

## 1. Transform Rule: response headers

**Rules → Create rule → Response Header Transform Rule** (Cloudflare's
current name for modify-response-header rules).

- Rule name: e.g. `Security headers`
- When incoming requests match: **All incoming requests** (this is a
  single-purpose static site — every response gets the same headers)
- Then: **Set static** for each of the following

### `Content-Security-Policy`

```
default-src 'none'; script-src 'self' 'sha256-R04+76miHHnkN/gWcsxD3a9pWM52m8KuVPq/ujEngqE=' 'sha256-VW0DRQl3KZ97qykJSzGadSViC3KuBUzcUoADouFhSk8=' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
```

Directive by directive:

- `default-src 'none'` — deny-by-default baseline. Every resource this
  site ever loads is same-origin (fonts, CSS, images, wasm), so nothing
  needs a broader fallback.
- `script-src 'self' 'sha256-...' 'sha256-...' 'wasm-unsafe-eval'` —
  scripts must come from the same origin, or be one of the site's two
  known inline scripts (theme-restore in `<head>`, theme-toggle in the
  rail — see `security/csp-hashes.txt`). `'wasm-unsafe-eval'` is required
  because both demo pages (`/demos/reaction-diffusion/`,
  `/demos/aho-corasick/`) call `WebAssembly.instantiateStreaming`, which
  browsers gate behind this token even though it's a normal, safe wasm
  load path (not `eval`-style string execution).
- `style-src 'self'` — all CSS is the one same-origin stylesheet
  (`/style.css`); no inline `style=""` attributes or `<style>` blocks
  ship anywhere (verified by `grep -rc 'style="' dist` in CI — see
  `.github/workflows/deploy.yml`), so no `'unsafe-inline'` is needed.
- `img-src 'self'` — all images (favicon, apple-touch-icon, og-image,
  resume page SVGs) are same-origin.
- `font-src 'self'` — both webfonts (`InterVariable.woff2`,
  `JetBrainsMono.woff2`) are served from `/fonts/`, same-origin.
- `connect-src 'self'` — the wasm demos fetch their `.wasm` binaries via
  `WebAssembly.instantiateStreaming(fetch(...))`, a same-origin network
  request that `connect-src` (not `script-src`) governs.
- `base-uri 'none'` — nothing on the site needs a `<base>` tag; this
  closes off base-tag injection as an attack vector.
- `form-action 'none'` — the site has no forms.
- `frame-ancestors 'none'` — the site should never be framed by another
  origin (clickjacking defense; the modern replacement for
  `X-Frame-Options`).

### `X-Content-Type-Options`

```
nosniff
```

Stops browsers from MIME-sniffing responses into an unintended content
type (e.g. treating a response as executable script when it isn't).

### `Referrer-Policy`

```
strict-origin-when-cross-origin
```

Sends the full URL as referrer for same-origin navigation, but only the
origin (no path) cross-origin — reasonable default that doesn't leak
page paths to third parties while still letting analytics on other sites
see where a click came from.

### `Permissions-Policy`

```
camera=(), microphone=(), geolocation=()
```

The site uses none of these; disable them explicitly so an embedded
third party (there shouldn't be one, but defense in depth) can't request
them either.

## 2. HSTS

**As applied: a fifth header in the same Transform Rule** — simplest, and
keeps all five headers reviewable in one place:

### `Strict-Transport-Security`

```
max-age=31536000
```

(Cloudflare's built-in toggle under **SSL/TLS → Edge Certificates →
HTTP Strict Transport Security** sets the identical header and is a fine
alternative — use one mechanism, not both, or the header duplicates.)

Settings rationale, whichever mechanism:

- **Max age**: 12 months
- **Apply HSTS policy to subdomains (includeSubDomains)**: **off**
  initially. Reason: turning this on commits every current and future
  subdomain of `paul-maxwell.com` to HTTPS-only, permanently (browsers
  cache it for the max-age). If a subdomain is ever stood up that can't
  do HTTPS yet — a quick redirect target, a third-party CNAME during
  setup, anything — `includeSubDomains` bricks it for every visitor who
  already has the policy cached, with no fast way to undo it. Revisit
  this once every subdomain that exists or is planned is confirmed
  HTTPS-only; there's no urgency to turn it on early.
- **Preload**: off. Preload submission is effectively permanent (it ships
  in browser source trees) and is only worth doing after `includeSubDomains`
  has been on, stable, and uneventful for a while.

## 3. Verify afterwards

```
curl -sI https://paul-maxwell.com/ | grep -i -E 'content-security-policy|x-content-type-options|referrer-policy|permissions-policy|strict-transport-security'
```

Expected (wrapped for readability; the real header is one line):

```
content-security-policy: default-src 'none'; script-src 'self' 'sha256-R04+76miHHnkN/gWcsxD3a9pWM52m8KuVPq/ujEngqE=' 'sha256-VW0DRQl3KZ97qykJSzGadSViC3KuBUzcUoADouFhSk8=' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
x-content-type-options: nosniff
referrer-policy: strict-origin-when-cross-origin
permissions-policy: camera=(), microphone=(), geolocation=()
strict-transport-security: max-age=31536000
```

**Warning:** a typo in the CSP (wrong hash, missing `'self'`, a stray
character) doesn't produce an error page — it silently blocks the
site's JS. The theme toggle and both wasm demos degrade completely
invisibly on a bad CSP. After applying the rule, load the site with the
browser devtools console open and check for CSP violation messages, then
manually exercise:

- the dark-mode toggle button (rail),
- `/demos/reaction-diffusion/` (canvas should render and respond to
  input, not stay blank),
- `/demos/aho-corasick/` (same).

## 4. Keeping this in sync

`security/csp-hashes.txt` is the source of truth for the `script-src`
hash values above, and `scripts/check-csp-hashes.sh` (run in CI on every
build, see `.github/workflows/deploy.yml`) fails the build if the hashes
in that file ever stop matching what the built site actually ships.

That CI check only guards the repo side. It cannot see or update the
Cloudflare rule. If either inline script's bytes ever change (a hash in
`security/csp-hashes.txt` changes as a result), **the Cloudflare
Transform Rule's CSP must be updated with the new hash in the same
change** — otherwise the next deploy ships a script the edge CSP no
longer allows, and it silently stops running for every visitor.
