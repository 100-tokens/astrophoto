# EDGE_CASES.md

Matrice de cas-limites **surface × catégorie**. Surfaces dérivées du dépôt
(`frontend/src/routes/**/+page.svelte`, `frontend/src/routes/**/+server.*`,
`frontend/src/lib/server/**` — ce dernier est vide). Catégories fixes (8).

Couverture vérifiée par `scripts/check-edge-cases.mjs` : chaque cellule
(surface × catégorie) porte ≥ 1 cas concret **ou** une mention `N/A: <raison>`
(bloc `@na`).

Catégories : `entrées-limites`, `états`, `concurrence`, `auth-tenant`,
`intégrité-données`, `contrat-API`, `frontend`, `sécurité`.

Format d'un cas :

```
@case
id: FE-0001
surface: /upload
catégorie: entrées-limites
tag: front
précondition: ...
action: ...
attendu: ...
@end
```

## Surfaces

| key | tag | description |
|-----|-----|-------------|
| / | front | Accueil / feed explore |
| /about | front | Page statique « à propos » |
| /account/frames | front | Bibliothèque de frames de calibration du compte |
| /admin/equipment/[id] | front | Édition d'un équipement (admin) |
| /admin/equipment | front | Liste équipements (admin) |
| /admin/settings | front | Réglages applicatifs (admin) |
| /c/[cat] | front | Feed par catégorie |
| /contact | front | Page de contact |
| /design | front | Galerie de composants / design system |
| /email-change/[token] | front | Confirmation de changement d'email par token |
| /equip/[kind]/[slug] | front | Fiche publique d'un équipement |
| /equip/[kind]/[slug]/edit | front | Édition de la fiche équipement |
| /equip/[kind] | front | Index des équipements d'un type |
| /explore | front | Feed explore filtrable |
| /me/drafts | front | Brouillons de l'utilisateur |
| /photographers | front | Annuaire des photographes |
| /privacy | front | Politique de confidentialité (statique) |
| /reset/[token] | front | Saisie du nouveau mot de passe via token |
| /reset | front | Demande de réinitialisation de mot de passe |
| /reset/sent | front | Accusé d'envoi du mail de reset |
| /search | front | Recherche globale |
| /settings/appearance | front | Préférences d'apparence (thème) |
| /settings/delete | front | Suppression de compte |
| /settings/email | front | Changement d'adresse email |
| /settings/equipment/[id]/edit | front | Édition d'un équipement utilisateur |
| /settings/equipment | front | Liste des équipements de l'utilisateur |
| /settings/equipment/new | front | Création d'un équipement utilisateur |
| /settings/password | front | Changement de mot de passe |
| /settings/profile | front | Édition du profil |
| /settings/sessions | front | Gestion des sessions actives |
| /settings/tokens | front | Gestion des PAT (tokens d'accès personnels) |
| /signin | front | Connexion |
| /signup | front | Inscription |
| /signup/check-email | front | Accusé d'inscription, vérification email |
| /t/[slug] | front | Page d'une cible céleste |
| /t | front | Index des cibles |
| /tag/[slug] | front | Feed par tag |
| /terms | front | Conditions d'utilisation (statique) |
| /u/[handle] | front | Profil public d'un photographe |
| /u/[handle]/p/[shortid] | front | Permalien d'une photo |
| /upload/[id]/verify | front | Vérification post-upload (plate-solve) |
| /upload | front | Upload simple (presigned PUT → finalize → publish) |
| /upload/batch | front | Upload par lot |
| /upload/batch/edit | front | Édition des métadonnées du lot |
| /account/logout | back | Endpoint de déconnexion (+server.ts) |
| /api/[...rest] | back | Reverse-proxy navigateur → API axum (+server.ts) |
| /robots.txt | back | robots.txt généré (+server.ts) |
| /rss.xml | back | Flux RSS généré (+server.ts) |
| /sitemap.xml | back | Sitemap généré (+server.ts) |

## Cas

<!-- Les blocs @case et @na sont ajoutés ci-dessous, regroupés par surface. -->

<!-- ===== batch A ===== -->

@case
id: FE-0100
surface: /signin
catégorie: entrées-limites
tag: front
précondition: page /signin chargée, déconnecté
action: soumettre le formulaire avec email vide et password rempli
attendu: l'action default de /signin/+page.server.ts retourne fail(400, {message:'Email and password are required.'}) sans appeler /api/auth/login; le message s'affiche via form.message
@end

@case
id: FE-0101
surface: /signin
catégorie: entrées-limites
tag: back
précondition: aucun compte n'existe pour cette adresse
action: POST /api/auth/login avec un email syntaxiquement malformé "a@@b" et un mot de passe quelconque
attendu: login::handler appelle queries::find_by_email (aucun résultat), exécute password::verify_dummy() puis renvoie AppError::Unauthorized (401); le front affiche 'Invalid email or password.'
@end

@case
id: FE-0102
surface: /signin
catégorie: entrées-limites
tag: back
précondition: déconnecté
action: POST /api/auth/login avec un champ email de 100000 caractères
attendu: LoginBody désérialise sans contrainte de longueur; find_by_email ne matche pas, verify_dummy() s'exécute, 401 Unauthorized renvoyé — pas de panique ni 500
@end

@case
id: FE-0103
surface: /signin
catégorie: états
tag: back
précondition: compte existant dont email_verified_at IS NULL, bon mot de passe
action: POST /api/auth/login avec les identifiants corrects
attendu: login::handler vérifie le hash, login_throttle::clear, puis détecte email_verified_at.is_none() et renvoie AppError::Forbidden (403)
@end

@case
id: FE-0104
surface: /signin
catégorie: états
tag: front
précondition: compte existant non vérifié, identifiants corrects
action: soumettre le formulaire /signin
attendu: l'action voit res.status === 403 et fait throw redirect(303, /signup/check-email?email=...) au lieu d'afficher une erreur inline
@end

@case
id: FE-0105
surface: /signin
catégorie: concurrence
tag: back
précondition: compte avec login_throttle.failed_count à 9 (MAX_FAILURES=10)
action: envoyer plusieurs tentatives de mauvais mot de passe en parallèle
attendu: login_throttle::record_failure upsert sur conflict(user_id); au franchissement de MAX_FAILURES un locked_until fixe (now()+15min) est posé une seule fois et failed_count repasse à 0; le lock n'est pas prolongé par les requêtes concurrentes
@end

@case
id: FE-0106
surface: /signin
catégorie: auth-tenant
tag: back
précondition: utilisateur déjà authentifié (locals.user présent)
action: naviguer vers /signin
attendu: le load de /signin/+page.server.ts fait throw redirect(303, '/') avant tout rendu du formulaire
@end

@case
id: FE-0107
surface: /signin
catégorie: auth-tenant
tag: back
précondition: compte verrouillé (login_throttle.locked_until > now()), bon mot de passe fourni
action: POST /api/auth/login avec les identifiants corrects
attendu: login_throttle::is_locked renvoie true, verify_dummy() s'exécute et 401 est renvoyé AVANT le verify réel — un bon mot de passe ne contourne pas un lock actif
@end

@case
id: FE-0108
surface: /signin
catégorie: intégrité-données
tag: back
précondition: compte OAuth-only (users.password_hash IS NULL)
action: POST /api/auth/login avec n'importe quel mot de passe
attendu: login::handler prend la branche user.password_hash None, exécute verify_dummy() et renvoie 401 — aucune session créée, aucune ligne sessions insérée
@end

@case
id: FE-0109
surface: /signin
catégorie: contrat-API
tag: back
précondition: identifiants invalides
action: POST /api/auth/login avec mauvais mot de passe pour un compte existant
attendu: login_throttle::record_failure puis AppError::Unauthorized → 401 avec enveloppe {error:"unauthorized", message:"unauthorized"}; le front mappe sur 'Invalid email or password.'
@end

@case
id: FE-0110
surface: /signin
catégorie: contrat-API
tag: back
précondition: identifiants corrects, compte vérifié
action: POST /api/auth/login réussi
attendu: réponse 200 Json(User) avec header set-cookie contenant la session; l'action front parse set-cookie et le repose côté origine frontend via cookies.set, puis redirect(303,'/')
@end

@case
id: FE-0111
surface: /signin
catégorie: frontend
tag: front
précondition: une action précédente a renvoyé fail avec form.message
action: observer le rendu après échec
attendu: le bloc {#if form?.message} affiche <p class="t-meta form-error">; les champs email/password sont required (validation HTML) et type=email/password
@end

@case
id: FE-0112
surface: /signin
catégorie: sécurité
tag: back
précondition: adresse inconnue vs adresse connue mais mauvais mot de passe
action: comparer le temps de réponse des deux POST /api/auth/login
attendu: chaque chemin 401 exécute exactement une vérification Argon2 (verify ou verify_dummy via DUMMY_HASH), donc le timing ne révèle pas l'existence du compte (anti-énumération documentée)
@end

@case
id: FE-0113
surface: /signin
catégorie: sécurité
tag: front
précondition: compte existant non vérifié
action: soumettre /signin avec identifiants corrects mais email non vérifié
attendu: le redirect 303 vers /signup/check-email?email=<adresse> révèle que l'adresse est enregistrée-mais-non-vérifiée — fuite d'énumération mineure sur le chemin 403 (à noter, distinct du chemin 401 protégé par verify_dummy)
@end

@case
id: FE-0114
surface: /signup
catégorie: entrées-limites
tag: front
précondition: page /signup chargée
action: soumettre avec un des quatre champs (display_name/handle/email/password) vide
attendu: l'action default retourne fail(400, {message:'All fields are required.'}) sans toucher /api/auth/signup
@end

@case
id: FE-0115
surface: /signup
catégorie: entrées-limites
tag: front
précondition: page /signup chargée
action: soumettre avec un password de 9 caractères
attendu: garde front password.length < 10 → fail(400, {message:'Password must be at least 10 characters.'}) avant tout appel réseau
@end

@case
id: FE-0116
surface: /signup
catégorie: entrées-limites
tag: back
précondition: signups_enabled = true
action: POST /api/auth/signup avec display_name de 101 caractères
attendu: SignupBody::validate échoue sur length(max=100) → AppError::Validation → 422; le front mappe 422 sur 'Please check your inputs.'
@end

@case
id: FE-0117
surface: /signup
catégorie: entrées-limites
tag: back
précondition: signups_enabled = true
action: POST /api/auth/signup avec handle "Marie" (majuscule)
attendu: crate::auth::handle::validate renvoie HandleError::Format (regex [a-z0-9_-], 3-30) → AppError::Validation → 422
@end

@case
id: FE-0118
surface: /signup
catégorie: états
tag: back
précondition: signups_enabled = false (réglage super-admin)
action: POST /api/auth/signup avec entrées valides
attendu: handler renvoie AppError::BadRequest("registration is currently closed") (400) avant le hash Argon2; le front affiche 'Sign-up failed: ...' via la branche 500/txt? non — 400 tombe dans le else txt → message brut
@end

@case
id: FE-0119
surface: /signup
catégorie: états
tag: back
précondition: handle libéré présent dans handle_redirects avec released_at > now() (cooldown 90j)
action: POST /api/auth/signup réutilisant ce handle
attendu: la requête scalar in_cooldown renvoie true → AppError::Conflict("handle is reserved") (409); le front affiche handleError 'That handle is already taken.'
@end

@case
id: FE-0120
surface: /signup
catégorie: concurrence
tag: back
précondition: deux requêtes signup simultanées avec le même handle libre
action: POST /api/auth/signup en parallèle avec un handle identique
attendu: l'index unique sur users.handle (backstop documenté) fait échouer le second insert dans queries::create_with_password → erreur unique → 409 Conflict; pas de doublon de ligne users
@end

@case
id: FE-0121
surface: /signup
catégorie: auth-tenant
tag: front
précondition: utilisateur déjà authentifié
action: naviguer vers /signup
attendu: load fait throw redirect(303, '/') (pas de formulaire d'inscription pour un connecté)
@end

@case
id: FE-0122
surface: /signup
catégorie: intégrité-données
tag: back
précondition: aucune adresse en doublon
action: POST /api/auth/signup avec un email déjà présent dans users
attendu: queries::create_with_password viole l'unicité email → 409 Conflict("email already in use"); le front, message sans 'handle', affiche 'An account with that email already exists.'
@end

@case
id: FE-0123
surface: /signup
catégorie: intégrité-données
tag: back
précondition: signups_enabled = true
action: POST /api/auth/signup avec password de 10 caractères figurant dans common-passwords.txt (ex. "1234567890")
attendu: signup n'appelle PAS password::validate_strength (seul reset le fait, min 12 + dictionnaire); SignupBody n'exige que length(min=10), donc le mot de passe faible est ACCEPTÉ et le compte créé — règle plus laxiste qu'au reset
@end

@case
id: FE-0124
surface: /signup
catégorie: contrat-API
tag: back
précondition: entrées valides, signups ouverts
action: POST /api/auth/signup réussi
attendu: handler renvoie 202 ACCEPTED avec Json{status:"verification_required", email}; AUCUN set-cookie; l'action front throw redirect(303, /signup/check-email?email=...)
@end

@case
id: FE-0125
surface: /signup
catégorie: contrat-API
tag: front
précondition: backend renvoie 409 avec {error:"conflict", message:"conflict: handle already taken"}
action: observer le mapping front du 409
attendu: l'action parse body.message, teste msg.includes('handle') pour distinguer handleError vs message email — désambiguïsation par texte de message (couplage au libellé backend)
@end

@case
id: FE-0126
surface: /signup
catégorie: frontend
tag: front
précondition: une soumission a échoué et renvoyé form.handle
action: observer la valeur du champ handle après round-trip serveur
attendu: le $effect réhydrate handle = form.handle (préservation via HandlePicker bind:value); form.handleError s'affiche sous le champ
@end

@case
id: FE-0127
surface: /signup
catégorie: sécurité
tag: back
précondition: signups ouverts, flux de masse
action: déclencher de nombreux signups en moins d'une heure
attendu: email_verify::global_cap_hit (GLOBAL_HOUR_CAP=200 sur email_verification_tokens) coupe l'émission du token de vérif; le compte est quand même créé et la réponse 202 reste inchangée (anti mail-bombing)
@end

@case
id: FE-0128
surface: /signup
catégorie: sécurité
tag: back
précondition: handle réservé statique présent dans data/reserved_handles.txt (ex. "admin")
action: POST /api/auth/signup avec handle "admin"
attendu: handle::validate renvoie HandleError::Reserved → AppError::Validation → 422 (impossible de squatter un handle de service)
@end

@case
id: FE-0129
surface: /signup/check-email
catégorie: entrées-limites
tag: front
précondition: page chargée avec ?email vide
action: soumettre l'action ?/resend avec le champ caché email vide
attendu: l'action resend retourne fail(400, {error:'missing_email'}) sans appeler /api/auth/resend-verification
@end

@case
id: FE-0130
surface: /signup/check-email
catégorie: états
tag: front
précondition: page chargée (?email présent)
action: observer le bouton Resend pendant les 60 premières secondes
attendu: le $effect lance un setInterval décrémentant secondsLeft; tant que secondsLeft>0 le Button est disabled et affiche 'Resend in {secondsLeft}s'
@end

@case
id: FE-0131
surface: /signup/check-email
catégorie: états
tag: back
précondition: compte déjà vérifié (email_verified_at non NULL)
action: soumettre ?/resend pour cette adresse
attendu: email_verify::resend voit u.email_verified_at.is_some() → ne ré-émet aucun token et renvoie 204 No Content; le front affiche tout de même 'If your account exists and isn't yet verified, we sent another link.'
@end

@case
id: FE-0132
surface: /signup/check-email
catégorie: concurrence
tag: back
précondition: un token de vérif vient d'être émis (< PER_EMAIL_COOLDOWN_SECS=60)
action: cliquer Resend de façon répétée / requêtes parallèles
attendu: email_verify::resend détecte cooldown_hit (token created_at < 60s) et ne ré-émet pas; renvoie 204 uniforme; le bouton front est par ailleurs disabled pendant le décompte
@end

@na
surface: /signup/check-email
catégorie: auth-tenant
raison: l'action ?/resend ne lit aucune session et n'exige aucune authentification (flux pré-vérification public); resend::handler ne porte pas de garde CurrentUser/SessionOnly et identifie l'utilisateur uniquement par l'email du corps
@end

@case
id: FE-0134
surface: /signup/check-email
catégorie: intégrité-données
tag: back
précondition: compte non vérifié, sous cap
action: soumettre ?/resend une fois passé le cooldown
attendu: email_verify::issue_token insère une ligne dans email_verification_tokens (token_hash sha256, expires_at now()+24h, used_at NULL); les anciens tokens ne sont pas supprimés mais restent à usage unique via used_at
@end

@case
id: FE-0135
surface: /signup/check-email
catégorie: contrat-API
tag: back
précondition: backend renvoie toujours 204 (anti-énumération)
action: soumettre ?/resend pour une adresse inexistante
attendu: /api/auth/resend-verification renvoie 204 No Content même sans compte; l'action front ignore le statut et retourne {ok:true}; aucune fuite d'existence
@end

@case
id: FE-0136
surface: /signup/check-email
catégorie: frontend
tag: front
précondition: URL /signup/check-email?email=<a href=x>&expired=1
action: charger la page avec ces paramètres
attendu: email = page.url.searchParams.get('email') rendu via {email} dans <strong>{email}</strong> — auto-échappement Svelte neutralise le XSS réfléchi; le bloc {#if expired} affiche l'avertissement de lien expiré
@end

@case
id: FE-0137
surface: /signup/check-email
catégorie: sécurité
tag: back
précondition: compte non vérifié, attaquant martèle resend
action: dépasser PER_HOUR_CAP=5 tokens/heure pour une adresse, ou GLOBAL_HOUR_CAP=200 site-wide
attendu: resend voit hour_cap_hit (count>=5 sur user_id) ou global_cap_hit (>=200) et supprime l'émission; renvoie 204 — anti mail-flooding, plafond par compte (clé user_id, pas IP proxy)
@end

@case
id: FE-0138
surface: /reset
catégorie: entrées-limites
tag: front
précondition: page /reset chargée
action: soumettre le formulaire avec email vide (espaces uniquement)
attendu: l'action trim l'email, voit !email et retourne fail(400, {error:'missing_email'}); le front affiche 'Please enter a valid email.'
@end

@case
id: FE-0139
surface: /reset
catégorie: états
tag: back
précondition: déploiement Koyeb qui ne fournit que BACKEND_URL (pas VITE_API_BASE_URL)
action: soumettre /reset avec une adresse valide
attendu: l'action utilise API = import.meta.env.VITE_API_BASE_URL ?? localhost:8080 (pas process.env.BACKEND_URL), donc l'appel échouerait vers localhost; le catch est avalé et redirect(303, /reset/sent) survient quand même — échec silencieux
@end

@case
id: FE-0140
surface: /reset
catégorie: concurrence
tag: back
précondition: un token de reset émis il y a < PER_EMAIL_COOLDOWN_SECS=60
action: re-soumettre /reset pour la même adresse en rafale
attendu: password_reset::request calcule cooldown_hit (token created_at < 60s) → n'émet pas de nouveau token; renvoie quand même 204 No Content (uniforme)
@end

@na
surface: /reset
catégorie: auth-tenant
raison: /api/auth/password-reset/request est public et volontairement sans session (un utilisateur ayant perdu son accès doit pouvoir l'appeler); request::handler n'a aucune garde CurrentUser et n'identifie la cible que par l'email du corps
@end

@case
id: FE-0142
surface: /reset
catégorie: intégrité-données
tag: back
précondition: compte existant, sous tous les plafonds
action: POST /api/auth/password-reset/request
attendu: une ligne password_reset_tokens est insérée (token_hash sha256, expires_at now()+1h via TTL_HOURS, used_at NULL, request_ip); le token brut n'est jamais stocké, seulement son sha256
@end

@case
id: FE-0143
surface: /reset
catégorie: contrat-API
tag: back
précondition: adresse inexistante
action: POST /api/auth/password-reset/request pour une adresse inconnue
attendu: request::handler trouve user None et renvoie 204 No Content sans rien émettre; le front redirect(303, /reset/sent?email=...) — réponse identique au cas existant (anti-énumération)
@end

@case
id: FE-0144
surface: /reset
catégorie: frontend
tag: front
précondition: action retourne fail avec error 'missing_email'
action: observer le rendu
attendu: {#if form?.error === 'missing_email'} affiche <p class="t-meta form-error">; champ email required type=email; lien retour vers /signin
@end

@case
id: FE-0145
surface: /reset
catégorie: sécurité
tag: back
précondition: attaquant unique tente de déclencher des reset en masse pour épuiser le quota d'autrui
action: déclencher > GLOBAL_HOUR_CAP=200 reset/heure, ou > PER_HOUR_CAP=5 pour une cible
attendu: le cap par compte est désormais clé sur user_id SEUL (plus OR'd avec request_ip proxy), donc un attaquant ne peut plus, via l'IP proxy unique, refuser la récupération à tous; global_cap_hit reste un plafond site-wide séparé non-OR
@end

@case
id: FE-0146
surface: /reset/[token]
catégorie: entrées-limites
tag: front
précondition: page /reset/<token> chargée
action: soumettre new_password de 11 caractères
attendu: garde front new_password.length < 12 → fail(400, {error:'too_short'}) avant tout appel; le front affiche 'Password must be at least 12 characters.'
@end

@case
id: FE-0147
surface: /reset/[token]
catégorie: entrées-limites
tag: back
précondition: token valide, mot de passe de 12 caractères figurant dans common-passwords.txt
action: POST /api/auth/password-reset/confirm
attendu: password::validate_strength renvoie "password_too_common" → AppError::bad_request → 400; le mot de passe faible est rejeté (contrôle absent côté signup)
@end

@case
id: FE-0148
surface: /reset/[token]
catégorie: états
tag: back
précondition: token déjà utilisé (used_at non NULL) ou expiré (expires_at < now())
action: POST /api/auth/password-reset/confirm avec ce token
attendu: confirm détecte row.used_at.is_some() || expires_at < now() → AppError::gone("expired_or_used") (410); le front affiche le panneau 'Link expired or already used'
@end

@case
id: FE-0149
surface: /reset/[token]
catégorie: états
tag: back
précondition: token totalement inexistant (jamais émis)
action: POST /api/auth/password-reset/confirm avec un token aléatoire
attendu: fetch_optional sur token_hash renvoie None → ok_or_else AppError::gone("expired_or_used") (410), même chemin que used/expired (pas d'état distinct invalid)
@end

@case
id: FE-0150
surface: /reset/[token]
catégorie: concurrence
tag: back
précondition: un token de reset valide et non utilisé
action: POST /api/auth/password-reset/confirm deux fois en parallèle avec le même token
attendu: confirm lit la ligne via fetch_optional SANS verrou (pas de "for update") et l'UPDATE used_at n'a pas de prédicat "and used_at is null" — les deux requêtes peuvent franchir le contrôle used_at et réussir; invariant usage-unique NON race-safe (contraste avec email_change qui utilise for update)
@end

@case
id: FE-0151
surface: /reset/[token]
catégorie: auth-tenant
tag: back
précondition: token de reset valide, attaquant non authentifié possédant le lien
action: POST /api/auth/password-reset/confirm
attendu: confirm supprime TOUTES les sessions (delete from sessions where user_id) puis crée une nouvelle session auto-login — la possession du lien suffit, par conception du flux de récupération; aucune session préalable requise
@end

@case
id: FE-0171
surface: /reset/[token]
catégorie: intégrité-données
tag: back
précondition: token valide
action: confirm réussi
attendu: dans une transaction unique: update users.password_hash + password_changed_at=now(), update password_reset_tokens.used_at=now(), delete sessions where user_id; tx.commit atomique — pas d'état partiel si une étape échoue
@end

@case
id: FE-0152
surface: /reset/[token]
catégorie: contrat-API
tag: back
précondition: token valide rejeté par validate_strength
action: POST confirm avec mot de passe trop commun
attendu: le backend renvoie l'enveloppe AppError {error:"bad-request", message:"bad request: password_too_common"} — il n'y a PAS de champ "code"; l'action front lit body.code (undefined) → detail='invalid', donc la branche front form.detail==='password_too_common' est du code mort jamais atteint
@end

@case
id: FE-0153
surface: /reset/[token]
catégorie: frontend
tag: front
précondition: utilisateur tape dans le champ new_password
action: saisir progressivement le mot de passe
attendu: la barre strength() (1<8, 2<12, 3<16, 4 sinon) s'affiche dès pwd.length>0 via .strength-seg.on; avertissement 'Use at least 12 characters' tant que pwd.length<12
@end

@case
id: FE-0154
surface: /reset/[token]
catégorie: sécurité
tag: back
précondition: token brut connu de l'attaquant
action: inspecter le stockage du token
attendu: le token brut n'est jamais persité; seul Sha256::digest(token) est stocké dans password_reset_tokens.token_hash, et confirm re-hash le token reçu pour le retrouver — une fuite de la table ne révèle pas les tokens
@end

@case
id: FE-0155
surface: /reset/sent
catégorie: entrées-limites
tag: front
précondition: navigation directe vers /reset/sent sans paramètre email
action: charger /reset/sent
attendu: email = page.url.searchParams.get('email') ?? '' → '' ; le titre affiche le fallback 'to your inbox.' et l'aperçu email rend To: vide sans planter
@end

@case
id: FE-0156
surface: /reset/sent
catégorie: états
tag: front
précondition: page chargée
action: attendre puis observer le bouton Resend
attendu: le $effect setInterval décrémente secondsLeft de 60→0; tant que >0, Button disabled 'Resend in 0:SS'; à 0, bouton actif appelant resend()
@end

@case
id: FE-0157
surface: /reset/sent
catégorie: concurrence
tag: front
précondition: secondsLeft est arrivé à 0
action: cliquer Resend rapidement plusieurs fois
attendu: resend() pose resending=true (Button disabled pendant l'appel api.passwordResetRequest), puis remet secondsLeft=60 — double-soumission bornée par le flag et le décompte
@end

@na
surface: /reset/sent
catégorie: auth-tenant
raison: page de confirmation purement publique; resend() appelle api.passwordResetRequest (endpoint anti-énumération public), aucune session ni propriété de ressource n'est vérifiée côté client ou serveur
@end

@na
surface: /reset/sent
catégorie: intégrité-données
raison: la page n'effectue aucune écriture DB propre; son unique appel réseau est resend → /api/auth/password-reset/request, dont l'insertion de password_reset_tokens et les invariants sont couverts par la cellule /reset intégrité-données (FE-0142)
@end

@case
id: FE-0160
surface: /reset/sent
catégorie: contrat-API
tag: front
précondition: resend() déclenché
action: cliquer Resend
attendu: api.passwordResetRequest poste /api/auth/password-reset/request {email}; le catch avale toute erreur et pose resentOk=true quoi qu'il arrive — le front ne révèle jamais si l'adresse existe
@end

@case
id: FE-0161
surface: /reset/sent
catégorie: frontend
tag: front
précondition: ?email contenant des entités HTML
action: charger /reset/sent?email="<img src=x>"
attendu: email est interpolé via {email} dans le <h1> et dans le <pre class="email-preview"> — l'auto-échappement Svelte rend le contenu littéral, pas d'exécution (XSS réfléchi neutralisé)
@end

@case
id: FE-0162
surface: /reset/sent
catégorie: sécurité
tag: front
précondition: page chargée
action: lire l'aperçu email statique affiché
attendu: le <pre> affiche un gabarit avec https://astrophoto.pics/reset/<your-token> littéral (placeholder), AUCUN vrai token n'est exposé dans le DOM — pas de fuite de token
@end

@case
id: FE-0163
surface: /email-change/[token]
catégorie: entrées-limites
tag: back
précondition: navigation vers /email-change/<token-aléatoire>
action: charger la page (déclenche le load)
attendu: load POST /api/auth/email-change/confirm {token}; confirm calcule sha256, fetch_optional ne trouve rien → renvoie ConfirmResponse{status:"expired"}; le front affiche le panneau 'Link expired'
@end

@case
id: FE-0164
surface: /email-change/[token]
catégorie: états
tag: back
précondition: token email-change expiré (expires_at <= now()) ou déjà utilisé (used_at non NULL)
action: charger /email-change/<token>
attendu: confirm: le match Some(r) if used_at.is_none() && expires_at>now() échoue → rollback → status:"expired"; used + expiré + inconnu sont fusionnés dans le même état "expired" (pas d'état invalid distinct)
@end

@case
id: FE-0165
surface: /email-change/[token]
catégorie: concurrence
tag: back
précondition: un token email-change valide non utilisé
action: charger /email-change/<token> deux fois en parallèle
attendu: confirm lit la ligne avec "for update" (verrou de ligne) dans la transaction, donc la seconde requête attend puis voit used_at non NULL → status:"expired"; invariant usage-unique race-safe (contraste avec password-reset)
@end

@case
id: FE-0166
surface: /email-change/[token]
catégorie: auth-tenant
tag: back
précondition: token valide émis pour user A
action: un tiers (user B connecté, ou anonyme) ouvre le lien /email-change/<token>
attendu: confirm n'exige aucune session (route /api/auth/email-change/confirm publique) et applique le changement au row.user_id encodé dans le token, pas à l'appelant — l'émission (request) exige CurrentUser+SessionOnly+mot de passe, mais la confirmation lie l'effet au propriétaire du token
@end

@case
id: FE-0167
surface: /email-change/[token]
catégorie: intégrité-données
tag: back
précondition: token valide, mais new_email déjà pris par un autre compte entre-temps
action: charger /email-change/<token>
attendu: l'UPDATE users.email déclenche is_unique_violation → rollback → ConfirmResponse{status:"taken"}; le front affiche 'Address already taken' — aucun email dupliqué inséré
@end

@case
id: FE-0168
surface: /email-change/[token]
catégorie: contrat-API
tag: back
précondition: token valide non utilisé, new_email libre
action: charger /email-change/<token>
attendu: confirm renvoie Json{status:"success"}; le load voit body.status==='success' et throw redirect(303, /settings/email?changed=1); le notification mail à l'ancienne adresse est best-effort (échec loggé, changement quand même commité)
@end

@case
id: FE-0169
surface: /email-change/[token]
catégorie: frontend
tag: front
précondition: confirm renvoie status 'taken'
action: observer le rendu
attendu: data.status==='taken' affiche le panel-danger 'Address already taken' avec bouton vers /settings/email; status 'error'/fallback affiche 'Something went wrong'
@end

@case
id: FE-0170
surface: /email-change/[token]
catégorie: sécurité
tag: back
précondition: l'email de changement contient le lien /email-change/<token>
action: un scanner de sécurité / préfetcheur de boîte mail récupère l'URL avant le clic humain
attendu: la confirmation s'exécute dans le load (GET sur navigation), donc une simple récupération de l'URL consomme le token usage-unique (used_at posé) — le vrai clic ultérieur obtient alors status:"expired"; mutation d'état sur GET
@end

<!-- ===== batch B ===== -->

@case
id: FE-0200
surface: /settings/password
catégorie: entrées-limites
tag: front
précondition: connecté, mot de passe local existant
action: soumettre new_password de 11 caractères dans password/+page.server.ts
attendu: court-circuit côté action `if (new_password.length < 12) return fail(400, { error: 'too_short' })` — l'appel POST /api/me/password-change n'est jamais émis
@end

@case
id: FE-0201
surface: /settings/password
catégorie: entrées-limites
tag: back
précondition: new_password = 12 espaces (passe le check FE de longueur)
action: POST /api/me/password-change avec new_password composé d'espaces
attendu: backend `password::validate_strength` compte 12 chars donc OK longueur, mais si présent dans common-passwords renvoie `password_too_common` → AppError::bad_request → 400, sinon accepté (validate_strength ne refuse que too_short/too_common)
@end

@case
id: FE-0202
surface: /settings/password
catégorie: entrées-limites
tag: back
précondition: new_password figurant dans assets/common-passwords.txt, ≥12 chars
action: POST /api/me/password-change
attendu: `validate_strength` renvoie "password_too_common" → AppError::bad_request → 400 ; +page.server.ts mappe `msg.includes('password_too_common')` → fail(400,{error:'too_common'})
@end

@case
id: FE-0203
surface: /settings/password
catégorie: états
tag: back
précondition: compte OAuth-only (users.password_hash IS NULL)
action: POST /api/me/password-change avec body { new_password } sans current_password
attendu: password_change::change saute la branche `if let Some(stored_hash) = user.password_hash` et accepte → 204, rotation de session
@end

@case
id: FE-0204
surface: /settings/password
catégorie: concurrence
tag: back
précondition: deux onglets A et B authentifiés sur la même session
action: l'onglet A change le mot de passe (delete from sessions where user_id, puis create_session), puis l'onglet B resoumet avec l'ancien cookie
attendu: l'ancienne session de B est supprimée → CurrentUser → AppError::Unauthorized → 401 → +page.server.ts mappe res.status===401 vers fail(401,{error:'wrong_password'})
@end

@case
id: FE-0205
surface: /settings/password
catégorie: auth-tenant
tag: back
précondition: client natif PixInsight authentifié par Bearer PAT (Authorization: Bearer astrophoto_pat_…)
action: POST /api/me/password-change avec ce PAT
attendu: l'extracteur `SessionOnly` voit l'extension `TokenAuth` posée par resolve() → AppError::Forbidden → 403 (jamais d'escalade compte via token)
@end

@case
id: FE-0206
surface: /settings/password
catégorie: intégrité-données
tag: back
précondition: session valide, mot de passe correct
action: POST /api/me/password-change
attendu: dans une seule tx : `update users set password_hash=$1, password_changed_at=now()` PUIS `delete from sessions where user_id=$1`, commit atomique ; nouvelle session créée APRÈS commit via session::create_session — si le commit échoue, ni le hash ni la purge de sessions ne s'appliquent
@end

@case
id: FE-0207
surface: /settings/password
catégorie: contrat-API
tag: back
précondition: backend saturé (semaphore Argon2, 8 permits, timeout 5s)
action: POST /api/me/password-change pendant une rafale d'auth
attendu: argon2_slot timeout → AppError::ServiceUnavailable → 503 ; +page.server.ts tombe sur le `return fail(500,{error:'server'})` (503 non mappé explicitement) → message "Something went wrong"
@end

@case
id: FE-0208
surface: /settings/password
catégorie: frontend
tag: front
précondition: changement réussi (form.ok)
action: rendu du formulaire après action
attendu: bloc `{#if form?.ok}` affiche "Password changed. Other devices have been signed out." ; le cookie de session roté est ré-injecté par cookies.set(name,value,opts) en parsant le Set-Cookie backend
@end

@case
id: FE-0209
surface: /settings/password
catégorie: sécurité
tag: back
précondition: compte OAuth-only sans mot de passe
action: vol d'un PAT et tentative de définir un mot de passe sans connaître l'ancien
attendu: bloqué par `_session_only: SessionOnly` → 403 ; commentaire du handler : "PATs must not change passwords — OAuth-only accounts skip the current-password check, so a stolen token would suffice"
@end

@case
id: FE-0210
surface: /settings/email
catégorie: entrées-limites
tag: back
précondition: connecté avec mot de passe, new_email sans '@'
action: POST /api/me/email-change/request new_email="notanemail"
attendu: email_change::request `if !new_email.contains('@')` → AppError::bad_request("invalid_email") → 400 ; action mappe `msg.includes('invalid_email')` → fail(400,{error:'invalid_email'})
@end

@case
id: FE-0211
surface: /settings/email
catégorie: états
tag: back
précondition: compte OAuth-only (password_hash NULL)
action: POST /api/me/email-change/request
attendu: `user.password_hash.ok_or_else(|| AppError::bad_request("no_password_set"))` → 400 "no_password_set" ; action retombe sur fail(400,{error:'invalid'}) (no_password_set non mappé spécifiquement)
@end

@case
id: FE-0212
surface: /settings/email
catégorie: concurrence
tag: back
précondition: une demande de changement émise il y a <60s
action: POST /api/me/email-change/request à nouveau
attendu: `cooldown_hit` (email_change_tokens.created_at > now()-60s) → AppError::too_many_requests("email_change_throttled") → 429 → fail(429,{error:'throttled'})
@end

@case
id: FE-0213
surface: /settings/email
catégorie: auth-tenant
tag: back
précondition: requête portée par un Bearer PAT
action: POST /api/me/email-change/request
attendu: guard `_session_only: SessionOnly` → TokenAuth présent → AppError::Forbidden → 403 (commentaire "Account-control endpoint: browser sessions only, never PATs")
@end

@case
id: FE-0214
surface: /settings/email
catégorie: intégrité-données
tag: back
précondition: deux comptes ; l'utilisateur B confirme un email déjà pris par A
action: POST /api/auth/email-change/confirm avec un token valide ciblant un email existant
attendu: `update users set email=$1` lève une unique_violation → tx.rollback() → Json(ConfirmResponse{status:"taken"}) ; le token n'est PAS marqué used_at (rollback) donc l'email courant reste inchangé
@end

@case
id: FE-0215
surface: /settings/email
catégorie: contrat-API
tag: back
précondition: token de changement expiré (>1h, TTL_HOURS=1) ou déjà used_at
action: POST /api/auth/email-change/confirm
attendu: branche `Some(r) if r.used_at.is_none() && r.expires_at > now()` non satisfaite → 200 OK avec corps `{status:"expired"}` (PAS un 4xx — statut délibérément 200 pour l'écran de confirmation)
@end

@case
id: FE-0216
surface: /settings/email
catégorie: frontend
tag: front
précondition: requestChange réussi (form.ok)
action: soumission de la modale Change email
attendu: `{#if form?.ok}` affiche "Check your new inbox for a confirmation link." ; la modale `<Modal bind:open={showModal}>` reste pilotée par l'état local $state(false)
@end

@case
id: FE-0217
surface: /settings/email
catégorie: sécurité
tag: back
précondition: changement d'email confirmé avec succès
action: confirm() commit la nouvelle adresse
attendu: notification envoyée à l'ANCIENNE adresse (old.email) avec email masqué via templates::mask_email ; si l'envoi échoue, `tracing::warn!("…change still committed")` — la sécurité repose sur l'alerte à l'ancien propriétaire, le changement n'est pas annulé
@end

@case
id: FE-0218
surface: /settings/email
catégorie: sécurité
tag: back
précondition: mot de passe courant erroné
action: POST /api/me/email-change/request avec current_password faux
attendu: `password::verify` (dans spawn_blocking, CPU-bound) renvoie false → AppError::Unauthorized → 401 → fail(401,{error:'wrong_password'}) ; vérification AVANT toute écriture de token
@end

@case
id: FE-0219
surface: /settings/profile
catégorie: entrées-limites
tag: back
précondition: connecté
action: PUT /api/me/profile display_name de 61 caractères
attendu: profile::put `trimmed.chars().count() > MAX_DISPLAY_NAME_CHARS (60)` → AppError::bad_request("invalid_display_name") → 400
@end

@case
id: FE-0220
surface: /settings/profile
catégorie: entrées-limites
tag: back
précondition: connecté
action: PUT /api/me/profile display_name = "   " (espaces uniquement)
attendu: `trimmed.is_empty()` → AppError::bad_request("invalid_display_name") → 400 (la valeur vide est rejetée, pas écrasée silencieusement)
@end

@case
id: FE-0221
surface: /settings/profile
catégorie: états
tag: front
précondition: HandleField, handle courant = "orion"
action: éditer le champ vers "orion" (inchangé après lowercase/trim)
attendu: `dirty = $derived(normalized !== current.toLowerCase())` = false → avail='current', `canSave` faux → le bouton Save reste inactif, aucun POST /api/me/handle
@end

@case
id: FE-0222
surface: /settings/profile
catégorie: concurrence
tag: front
précondition: AutosaveField display_name, frappes rapides
action: taper plusieurs caractères en <600ms
attendu: `onInput` réarme `debounceTimer = setTimeout(save, 600)` à chaque frappe ; un seul POST action="" déclenché après la pause — pas de POST par frappe
@end

@case
id: FE-0223
surface: /settings/profile
catégorie: auth-tenant
tag: front
précondition: non authentifié
action: naviguer vers /settings/profile
attendu: settings/+layout.server.ts `if (!locals.user) redirect(303, /signin?next=/settings/profile)` — redirection 303 avant tout chargement de page
@end

@case
id: FE-0259
surface: /settings/profile
catégorie: auth-tenant
tag: back
précondition: Bearer PAT (scope publish)
action: PUT /api/me/profile via PAT
attendu: profile::put N'EXIGE PAS SessionOnly (signature: State + CurrentUser + Json) → un PAT valide peut modifier le profil ; contraste avec password/email/delete/tokens qui imposent SessionOnly — le profil n'est pas un endpoint de contrôle-compte
@end

@case
id: FE-0224
surface: /settings/profile
catégorie: intégrité-données
tag: back
précondition: PUT contenant plusieurs sections (display_name, tagline, location)
action: PUT /api/me/profile multi-colonnes
attendu: profile::put valide TOUT avant écriture, puis `state.pool.begin()` exécute les UPDATE conditionnels par colonne et tx.commit() — atomique (si bortle_class hors 1..=9 → bad_request("bortle_out_of_range") AVANT toute écriture)
@end

@case
id: FE-0225
surface: /settings/profile
catégorie: contrat-API
tag: back
précondition: bio_html avec balises script
action: PUT /api/me/profile bio_html malveillant
attendu: `bio::sanitize` (ammonia) nettoie AVANT écriture ; cap à MAX_BIO_HTML_BYTES (16384) ; succès → 204 No Content
@end

@case
id: FE-0226
surface: /settings/profile
catégorie: frontend
tag: front
précondition: AutosaveField, réponse fetch non-ok
action: save() reçoit r.ok === false
attendu: `if (!r.ok) throw` → catch pose `error = true` → rendu `{#if error}<span class="err">● Save failed — retry</span>` ; l'état saved n'est pas affiché
@end

@case
id: FE-0227
surface: /settings/profile
catégorie: sécurité
tag: front
précondition: HandleField, vérification de disponibilité
action: taper un handle au format invalide "ab" (<3 chars) ou "Hé!"
attendu: `if (!/^[a-z0-9_-]{3,30}$/.test(h))` → avail='invalid' SANS appel réseau ; le POST /api/me/handle n'est tenté que si canSave (avail==='available')
@end

@case
id: FE-0228
surface: /settings/appearance
catégorie: entrées-limites
tag: back
précondition: connecté, valeur theme falsifiée
action: POST ?/setTheme avec theme="neon" (hors enum) qui appelle api.putPreferences
attendu: preferences::put `if t != "dark" && t != "light"` → AppError::bad_request("invalid_theme") → 400 ; mais le cookie 'theme' a DÉJÀ été posé par cookies.set avant syncToBackend → divergence cookie/DB
@end

@case
id: FE-0229
surface: /settings/appearance
catégorie: états
tag: front
précondition: visiteur anonyme (pas de session)
action: POST ?/setTheme
attendu: syncToBackend attrape `e instanceof ApiError && e.status === 401` et NE relance PAS ; le cookie 'theme' persiste (maxAge 1 an) ; redirect(303,'/settings/appearance') — le thème marche sans compte
@end

@case
id: FE-0230
surface: /settings/appearance
catégorie: concurrence
tag: front
précondition: deux onglets changeant theme/density
action: onglet A pose theme=dark, onglet B pose density=data quasi-simultanément
attendu: preferences::put utilise `set theme=coalesce($1,theme), density=coalesce($2,density)` — chaque écriture ne touche que sa colonne (NULL → coalesce conserve l'autre), pas d'écrasement croisé
@end

@case
id: FE-0231
surface: /settings/appearance
catégorie: auth-tenant
tag: back
précondition: Bearer PAT
action: PUT /api/me/preferences via PAT
attendu: preferences::put n'impose PAS SessionOnly (State + CurrentUser + Json) → un PAT peut écrire les préférences ; non considéré endpoint de contrôle-compte
@end

@na
surface: /settings/appearance
catégorie: intégrité-données
raison: theme/density sont deux colonnes scalaires sur users mises à jour par un seul UPDATE coalesce (preferences::put) ; aucune table de jonction, aucun cache dénormalisé, aucune cascade — il n'y a pas d'invariant multi-lignes à enfreindre.
@end

@case
id: FE-0232
surface: /settings/appearance
catégorie: contrat-API
tag: front
précondition: setTheme réussi
action: POST ?/setTheme valide
attendu: l'action termine par `throw redirect(303, '/settings/appearance')` — rechargement de la page (et de load → locals.preferences) plutôt que retour d'un form result ; le chip actif suit `data.preferences.theme === 'dark'`
@end

@case
id: FE-0233
surface: /settings/appearance
catégorie: frontend
tag: front
précondition: preferences.density='work'
action: rendu de la Row DENSITY
attendu: le bouton WORK reçoit `class:active={data.preferences.density === 'work'}` ; les chips sont des boutons name=density value=work|data dans `<form action="?/setDensity">`
@end

@na
surface: /settings/appearance
catégorie: sécurité
raison: les cookies 'theme' et 'density' sont volontairement non-secrets (COOKIE_OPTS httpOnly:false, sameSite:'lax') pour permettre au CSS de les lire côté client ; ils ne contiennent ni identifiant ni jeton, donc l'exposition JS est sans risque par conception.
@end

@case
id: FE-0234
surface: /settings/delete
catégorie: entrées-limites
tag: front
précondition: connecté, phrase de confirmation ≠ exacte
action: saisir "delete my account" (minuscules) dans confirmation_phrase
attendu: bouton `disabled={phrase !== 'DELETE MY ACCOUNT'}` reste désactivé ; et côté action `if (phrase !== 'DELETE MY ACCOUNT') return fail(400,{error:'phrase'})`
@end

@case
id: FE-0235
surface: /settings/delete
catégorie: états
tag: back
précondition: suppression déjà programmée (pending_deletion_at non NULL)
action: POST /api/me/delete-request à nouveau
attendu: `update users set pending_deletion_at=now()+7days where id=$1 and pending_deletion_at is null returning …` → fetch_optional renvoie None → pas de nouvel email, 204 idempotent (la date initiale n'est pas repoussée)
@end

@case
id: FE-0236
surface: /settings/delete
catégorie: concurrence
tag: back
précondition: pending_deletion_at non NULL
action: deux requêtes ?/cancel simultanées
attendu: deletion::cancel `update … set pending_deletion_at=null where id=$1 and pending_deletion_at is not null returning …` ; seule la première trouve une ligne (Some → email d'annulation), la seconde renvoie None → 204 sans double email
@end

@case
id: FE-0237
surface: /settings/delete
catégorie: auth-tenant
tag: back
précondition: Bearer PAT
action: POST /api/me/delete-request via PAT
attendu: guard `_session_only: SessionOnly` → TokenAuth présent → AppError::Forbidden → 403 (un token volé ne peut pas programmer la suppression du compte)
@end

@case
id: FE-0238
surface: /settings/delete
catégorie: intégrité-données
tag: back
précondition: compte avec mot de passe local
action: POST /api/me/delete-request, phrase OK mais mot de passe faux
attendu: deletion::request vérifie `password::verify` AVANT l'UPDATE de pending_deletion_at ; échec → AppError::Unauthorized → 401 → aucun changement de pending_deletion_at (la grâce de 7 jours n'est posée que sur preuve d'identité)
@end

@case
id: FE-0239
surface: /settings/delete
catégorie: contrat-API
tag: front
précondition: requestDeletion réussi
action: action ?/request retourne sans erreur
attendu: `redirect(303,'/settings/delete')` ; au rechargement, load lit `locals.user?.pending_deletion_at` → le panneau "● DELETION SCHEDULED" remplace le formulaire
@end

@case
id: FE-0240
surface: /settings/delete
catégorie: frontend
tag: front
précondition: pending_deletion_at posé
action: rendu de la branche `{#if data.pending_deletion_at}`
attendu: affiche la date via `new Date(data.pending_deletion_at).toLocaleString()`, un form ?/cancel "Cancel deletion · keep my account" et un lien d'export `${VITE_API_BASE_URL}/api/me/export.json` (download)
@end

@case
id: FE-0241
surface: /settings/delete
catégorie: sécurité
tag: back
précondition: compte OAuth-only (password_hash NULL)
action: POST /api/me/delete-request sans current_password
attendu: la branche `if let Some(stored) = row.password_hash` est sautée → suppression programmée sans mot de passe ; la phrase de confirmation "DELETE MY ACCOUNT" reste l'unique garde — re-auth par mot de passe seulement si un hash existe
@end

@case
id: FE-0242
surface: /settings/sessions
catégorie: entrées-limites
tag: back
précondition: connecté
action: DELETE /api/me/sessions/:id avec un id non-base64url (ex. "!!!")
attendu: users::sessions::revoke `URL_SAFE_NO_PAD.decode(&id_hex).map_err(|_| AppError::bad_request("bad_id"))` → 400 "bad_id"
@end

@case
id: FE-0243
surface: /settings/sessions
catégorie: états
tag: back
précondition: une seule session (la courante)
action: tenter de révoquer la session courante
attendu: revoke `if id == current_id return Err(AppError::bad_request("use_logout"))` → 400 ; l'UI ne montre PAS de bouton Revoke pour s.is_current (`{#if !s.is_current}`), donc accessible seulement par requête forgée
@end

@case
id: FE-0244
surface: /settings/sessions
catégorie: concurrence
tag: back
précondition: deux onglets, l'onglet A vient de révoquer la session de l'onglet B
action: l'onglet B (session supprimée) émet POST ?/signOutOthers
attendu: la session de B n'existe plus → CurrentUser → AppError::Unauthorized → 401 ; sign_out_others ne s'exécute jamais
@end

@case
id: FE-0245
surface: /settings/sessions
catégorie: auth-tenant
tag: back
précondition: utilisateur U1 connaît l'id de session de U2
action: DELETE /api/me/sessions/<id_de_U2>
attendu: `delete from sessions where id=$1 and user_id=$2` (user.id = U1) → rows_affected==0 → AppError::not_found("session") → 404 (impossible de révoquer la session d'autrui)
@end

@case
id: FE-0246
surface: /settings/sessions
catégorie: intégrité-données
tag: back
précondition: plusieurs sessions, une expirée
action: GET /api/me/sessions
attendu: list filtre `where user_id=$1 and expires_at > now()` et trie `(id=$2) desc, last_used_at desc` — la session courante (is_current via `(id=$2)`) apparaît en tête, les expirées sont exclues
@end

@case
id: FE-0247
surface: /settings/sessions
catégorie: contrat-API
tag: back
précondition: signOutOthers
action: POST /api/me/sessions/sign-out-others
attendu: `delete from sessions where user_id=$1 and id != $2` (préserve la courante) → StatusCode::NO_CONTENT (204) ; l'action SvelteKit retourne {ok:true}
@end

@case
id: FE-0248
surface: /settings/sessions
catégorie: frontend
tag: front
précondition: une seule session active (la courante)
action: rendu de la liste
attendu: `otherCount = $derived(data.sessions.filter(s => !s.is_current).length)` = 0 → le bloc "Sign out of all other sessions" `{#if otherCount > 0}` est masqué ; la ligne courante affiche "· this device"
@end

@case
id: FE-0249
surface: /settings/sessions
catégorie: sécurité
tag: back
précondition: session id affiché
action: inspecter l'id transmis au client
attendu: l'id de session brut (bytes) n'est jamais exposé en clair — il est encodé en base64 URL_SAFE_NO_PAD (`SessionRow.id`) ; le décodage côté revoke est la seule réintroprétation, et le secret de session (cookie) reste httpOnly côté __Host-session / session
@end

@case
id: FE-0250
surface: /settings/tokens
catégorie: entrées-limites
tag: back
précondition: nom de token = chaîne multi-octets de 80 codepoints comprenant des caractères >1 octet
action: POST /api/me/tokens (passe le maxlength="80" HTML qui compte en unités UTF-16)
attendu: tokens::create `if name.is_empty() || name.len() > 80` (bytes Rust) → un nom de 80 caractères multi-octets dépasse 80 bytes → AppError::bad_request("name") → 400 → fail(400,{error:'invalid'})
@end

@case
id: FE-0251
surface: /settings/tokens
catégorie: entrées-limites
tag: front
précondition: champ name vide ou espaces
action: soumettre ?/create avec name="   "
attendu: action `const name = String(...).trim(); if (!name) return fail(400,{error:'name_required'})` → rendu `form.error === 'name_required'` "Give the token a name." — POST backend non émis
@end

@case
id: FE-0252
surface: /settings/tokens
catégorie: états
tag: back
précondition: token déjà révoqué (revoked_at non NULL)
action: DELETE /api/me/tokens/:id sur ce token
attendu: revoke `update api_tokens set revoked_at=now() where id=$1 and user_id=$2 and revoked_at is null` → rows_affected==0 → AppError::not_found("token") → 404
@end

@case
id: FE-0253
surface: /settings/tokens
catégorie: concurrence
tag: front
précondition: liste de tokens affichée avec use:enhance
action: cliquer Revoke deux fois rapidement sur la même ligne
attendu: la 1re requête révoque (204), la 2e frappe le guard `revoked_at is null` → 404 → l'action retourne fail(500,{error:'server'}) (catch générique) ; invalidateAll via enhance re-rend la ligne en `(revoked)`
@end

@case
id: FE-0254
surface: /settings/tokens
catégorie: auth-tenant
tag: back
précondition: requête portée par un Bearer PAT (auto-gestion des tokens)
action: GET/POST/DELETE /api/me/tokens via PAT
attendu: les trois handlers (list/create/revoke) imposent `_session_only: SessionOnly` → TokenAuth présent → AppError::Forbidden → 403 (un token ne peut pas créer/révoquer d'autres tokens)
@end

@case
id: FE-0255
surface: /settings/tokens
catégorie: intégrité-données
tag: back
précondition: création de token
action: POST /api/me/tokens
attendu: seul le SHA-256 est persisté — `hash_secret` (Sha256::digest) stocké dans api_tokens.token_hash, le `prefix` (display_prefix, 20 chars) stocké séparément, scope='publish' ; le secret en clair n'est jamais en base
@end

@case
id: FE-0256
surface: /settings/tokens
catégorie: contrat-API
tag: back
précondition: révocation réussie
action: DELETE /api/me/tokens/:id valide
attendu: StatusCode::NO_CONTENT (204) ; l'id de chemin est typé `Path<Uuid>` — un id non-UUID est rejeté par l'extracteur axum avant le handler (400)
@end

@case
id: FE-0257
surface: /settings/tokens
catégorie: frontend
tag: front
précondition: aucun token (data.tokens.length === 0)
action: rendu de la liste
attendu: `{#if data.tokens.length === 0}<li class="empty">No tokens yet.</li>` — état vide explicite
@end

@case
id: FE-0258
surface: /settings/tokens
catégorie: sécurité
tag: front
précondition: token fraîchement créé (form.created présent)
action: rendu après ?/create
attendu: le secret complet `newToken.secret` n'est affiché qu'une fois dans le bloc reveal `{#if form && 'created' in form}` avec l'avertissement "This is the only time the full token is shown" ; les lignes existantes n'affichent que `t.prefix…`, jamais le secret (non stocké en clair)
@end

<!-- ===== batch C ===== -->

@case
id: FE-0300
surface: /settings/equipment/new
catégorie: entrées-limites
tag: back
précondition: authenticated user on SetupForm, all role pickers empty
action: submit with setupName = "   " (whitespace only)
attendu: POST /api/equipment/setups create.rs returns AppError::Validation("name is required") → 422 after input.name.trim().is_empty()
@end

@case
id: FE-0301
surface: /settings/equipment/new
catégorie: entrées-limites
tag: back
précondition: telescope role picker open in create mode, brand+model typed
action: SetupForm POSTs /api/equipment/items with brand="Sky-Watcher", model="   " (trimmed empty)
attendu: items_create::handler structured branch returns AppError::Validation("model is required") → 422
@end

@case
id: FE-0302
surface: /settings/equipment/new
catégorie: entrées-limites
tag: back
précondition: a camera item already exists with canonical "zwo asi2600mc"
action: type brand="ZWO" model="ASI2600MC" again and POST /api/equipment/items
attendu: items_create insert on conflict (kind, canonical_name) do nothing → union all re-selects existing row, usage_count unchanged (no duplicate)
@end

@case
id: FE-0303
surface: /settings/equipment/new
catégorie: états
tag: back
précondition: user already owns a setup flagged is_default=true
action: create a new setup with is_default=true
attendu: create.rs clears prior default (update equipment_setups set is_default=false where owner_id=$1 and is_default) in same tx before insert, so the partial unique idx never trips
@end

@case
id: FE-0304
surface: /settings/equipment/new
catégorie: concurrence
tag: back
précondition: two tabs both pick the not-yet-cataloged telescope "Askar FRA400"
action: both submit, each POSTing /api/equipment/items near-simultaneously
attendu: items_create on conflict (kind, canonical_name) do nothing makes the second insert a no-op; union all returns the row inserted by the first — no duplicate, both setups reference the same id
@end

@case
id: FE-0305
surface: /settings/equipment/new
catégorie: auth-tenant
tag: front
précondition: no session cookie
action: GET /settings/equipment/new
attendu: +page.server.ts load: if (!locals.user) redirect(303, '/signin')
@end

@case
id: FE-0306
surface: /settings/equipment/new
catégorie: intégrité-données
tag: back
précondition: SetupForm submits a role with item_id that is a valid uuid but absent from equipment_items
action: POST /api/equipment/setups with items[].item_id pointing at a nonexistent row
attendu: setup_items insert FK violation mapped by unknown_item_to_422 → 422 (not a 500), whole tx rolled back so the setup row is not orphaned
@end

@case
id: FE-0307
surface: /settings/equipment/new
catégorie: contrat-API
tag: back
précondition: SetupForm submits a role with an unrecognised role string
action: POST /api/equipment/setups with items[].role = "secondary_scope"
attendu: validate_role rejects before tx opens → AppError::Validation → 422 envelope {"error","message"}
@end

@case
id: FE-0308
surface: /settings/equipment/new
catégorie: frontend
tag: front
précondition: telescope picker open, EquipmentAutocomplete bound to the telescope kind
action: type "esprit 100" into the telescope EquipmentAutocomplete field, triggering GET /api/equipment/autocomplete?kind=telescope&q=esprit 100
attendu: backend ILIKE match on (brand || ' ' || model || coalesce(' '||variant,'')) surfaces "Sky-Watcher Esprit 100 ED" with specs_summary "100/550 f/5.5" rendered under the suggestion
@end

@case
id: FE-0309
surface: /settings/equipment/new
catégorie: sécurité
tag: back
précondition: attacker types a model name containing markup
action: POST /api/equipment/items with model="<img src=x onerror=alert(1)>"
attendu: stored verbatim in equipment_items.model; rendered as text by Svelte's default escaping in ComboBox cb-opt-val / SetupForm name — no XSS, server stores raw string with no sanitisation expected
@end

@case
id: FE-0310
surface: /settings/equipment/[id]/edit
catégorie: entrées-limites
tag: back
précondition: editing own setup, clear the name field then submit
action: PATCH /api/equipment/setups/:id with name="" 
attendu: update.rs returns AppError::Validation("name is required") → 422, items NOT yet deleted (validation precedes the delete from setup_items)
@end

@case
id: FE-0311
surface: /settings/equipment/[id]/edit
catégorie: états
tag: back
précondition: editing a non-default setup, user owns another default setup
action: PATCH /api/equipment/setups/:id with is_default=true
attendu: update.rs clears others (set is_default=false where owner_id=$1 and is_default and id <> $2) then sets this one default, in one tx
@end

@case
id: FE-0312
surface: /settings/equipment/[id]/edit
catégorie: concurrence
tag: back
précondition: two tabs open the same setup; tab A deletes it, tab B then saves edits
action: tab B PATCHes /api/equipment/setups/:id after the row is gone
attendu: select id ... for update returns None → AppError::NotFound("setup not found") → 404, no partial write
@end

@case
id: FE-0313
surface: /settings/equipment/[id]/edit
catégorie: auth-tenant
tag: back
précondition: user B authenticated, setup :id owned by user A
action: PATCH /api/equipment/setups/:id (user A's setup)
attendu: where id=$1 and owner_id=$2 for update finds nothing → AppError::NotFound → 404 (IDOR yields 404, not another user's setup)
@end

@case
id: FE-0314
surface: /settings/equipment/[id]/edit
catégorie: intégrité-données
tag: back
précondition: setup has 3 items; PATCH replaces with 2 items
action: PATCH /api/equipment/setups/:id with items=[telescope, camera]
attendu: update.rs delete from setup_items where setup_id=$1 then re-inserts the 2 — replace-all is atomic; equipment_items rows themselves untouched (only the junction is rewritten)
@end

@case
id: FE-0315
surface: /settings/equipment/[id]/edit
catégorie: contrat-API
tag: front
précondition: backend setups GET returns 404 for an unknown :id
action: load /settings/equipment/badid/edit
attendu: edit/+page.server.ts: if (r.status === 404) error(404, 'Setup not found'); other non-ok → error(500, 'Backend error')
@end

@case
id: FE-0316
surface: /settings/equipment/[id]/edit
catégorie: frontend
tag: front
précondition: setup item detail fetch (/api/equipment/items/:id) fails for one role
action: load prefill while one dr is !ok
attendu: edit/+page.server.ts falls back to { detail: null, item } so SetupForm still seeds name via detail?.display_name ?? item.display_name — page renders, no crash
@end

@case
id: FE-0317
surface: /settings/equipment/[id]/edit
catégorie: sécurité
tag: back
précondition: a session-only invariant — setups have no CSRF token, mutation is a fetch with credentials
action: cross-site fetch PATCH /api/equipment/setups/:id
attendu: blocked by SameSite=None + CORS single-origin APP_CORS_ORIGIN (per CLAUDE.md cookie policy); CurrentUser still required so anonymous cross-site gets 401
@end

@case
id: FE-0318
surface: /settings/equipment
catégorie: entrées-limites
tag: front
précondition: setDefault action invoked with no id field
action: POST ?/setDefault with empty form data
attendu: +page.server.ts: if (typeof id !== 'string' || !id) return fail(400, { error: 'Missing id' })
@end

@case
id: FE-0319
surface: /settings/equipment
catégorie: états
tag: front
précondition: user owns zero setups
action: load /settings/equipment, GET /api/equipment/setups returns []
attendu: setups: [] passed to page; list renders its empty state (no rows) without error
@end

@case
id: FE-0320
surface: /settings/equipment
catégorie: concurrence
tag: front
précondition: setDefault re-PATCHes the full setup using a freshly fetched detail
action: tab A deletes the setup, tab B clicks "set default" on the now-gone row
attendu: setDefault first GET /api/equipment/setups/:id returns !ok → fail(dr.status, { error: 'Could not load setup' }); no PATCH attempted
@end

@case
id: FE-0321
surface: /settings/equipment
catégorie: auth-tenant
tag: front
précondition: no session
action: GET /settings/equipment
attendu: +page.server.ts load: if (!locals.user) redirect(303, '/signin')
@end

@case
id: FE-0322
surface: /settings/equipment
catégorie: intégrité-données
tag: front
précondition: setDefault rebuilds the PATCH body from detail (name, items, default_apply_mode)
action: flip default on a setup
attendu: body.items = detail.items.map(it => ({ role, item_id: it.item.id })) and default_apply_mode preserved — no item or apply-mode loss on a default-flip
@end

@case
id: FE-0323
surface: /settings/equipment
catégorie: contrat-API
tag: front
précondition: delete action returns 204 from backend
action: POST ?/delete
attendu: +page.server.ts treats r.status !== 204 && !r.ok as failure; on 204 redirect(303, '/settings/equipment')
@end

@case
id: FE-0324
surface: /settings/equipment
catégorie: frontend
tag: front
précondition: GET /api/equipment/setups returns a 502/non-json gateway body
action: load /settings/equipment
attendu: if (!r.ok) error(500, 'Backend error') — surfaced as SvelteKit error page, not a thrown json parse
@end

@case
id: FE-0325
surface: /settings/equipment
catégorie: sécurité
tag: back
précondition: attacker passes another user's setup id to the delete action
action: POST ?/delete with id = victim's setup uuid
attendu: backend DELETE where id=$1 and owner_id=$2 → rows_affected()==0 → AppError::NotFound → 404; victim's setup untouched (IDOR-safe)
@end

@case
id: FE-0326
surface: /admin/equipment
catégorie: entrées-limites
tag: back
précondition: admin session
action: GET /api/admin/equipment?page=-5
attendu: admin::equipment::list: page = query.page.unwrap_or(0).max(0) → clamps negatives to 0; offset never negative
@end

@case
id: FE-0327
surface: /admin/equipment
catégorie: états
tag: front
précondition: catalog empty or q matches nothing
action: GET /admin/equipment?q=zzznomatch
attendu: list returns items:[], total:0, has_more:false; page renders empty result set
@end

@na
surface: /admin/equipment
catégorie: concurrence
raison: the list endpoint admin::equipment::list is read-only (count + select); it holds no locks and mutates nothing, so there is no two-writer race on this surface (mutations live on /admin/equipment/[id]).
@end

@case
id: FE-0329
surface: /admin/equipment
catégorie: auth-tenant
tag: back
précondition: authenticated NON-admin user
action: GET /api/admin/equipment
attendu: AdminUser extractor: Some(u) if !u.is_admin → AppError::Forbidden → 403; frontend +layout.server.ts also redirects non-admins to '/' (UX guard)
@end

@case
id: FE-0330
surface: /admin/equipment
catégorie: intégrité-données
tag: back
précondition: items exist with submitted_by referencing a deleted/null user
action: GET /api/admin/equipment
attendu: left join users u on u.id = e.submitted_by yields submitted_by_handle = null — list still returns the item rather than dropping it
@end

@case
id: FE-0331
surface: /admin/equipment
catégorie: contrat-API
tag: back
précondition: admin filters by an invalid kind value
action: GET /api/admin/equipment?kind=banana
attendu: list does NOT validate against VALID_KINDS — where kind = $1 simply matches zero rows, returns items:[] total:0 (200, not 422); documented divergence from autocomplete/catalog-values which 422
@end

@case
id: FE-0332
surface: /admin/equipment
catégorie: frontend
tag: front
précondition: list returns has_more=true at page 0
action: render pagination
attendu: has_more computed as offset + items.len() < total; page reads url.searchParams page/kind/q and forwards via fetchEquipment(fetch, {kind,q,page})
@end

@case
id: FE-0333
surface: /admin/equipment
catégorie: sécurité
tag: back
précondition: admin's q contains a SQL-special and a wildcard
action: GET /api/admin/equipment?q=%25_'; drop
attendu: q wrapped as format!("%{s}%") and passed as a bound $2 to ilike — parameterised, no injection; literal % in input just broadens the ILIKE match
@end

@case
id: FE-0334
surface: /admin/equipment/[id]
catégorie: entrées-limites
tag: back
précondition: admin editing an item, clears the model field
action: PATCH /api/admin/equipment/:id with model="   "
attendu: edit() computes model=trim → "" then returns AppError::Validation("model cannot be empty") → 422, tx rolled back (validation precedes the update)
@end

@case
id: FE-0335
surface: /admin/equipment/[id]
catégorie: états
tag: back
précondition: admin sets an out-of-set moderation status
action: PATCH /api/admin/equipment/:id with status="merged"
attendu: !EDITABLE_STATUSES.contains("merged") → AppError::Validation("status must be approved | pending | rejected") → 422 (merged reserved for deferred merge tooling)
@end

@case
id: FE-0336
surface: /admin/equipment/[id]
catégorie: concurrence
tag: back
précondition: admin A renames item to "Celestron EdgeHD 8" while another item already canonicalises to that name
action: PATCH /api/admin/equipment/:id producing a colliding canonical_name
attendu: update hits the (kind, canonical_name) unique constraint → Err with db.constraint() → AppError::Conflict("another item of this kind already uses that name") → 409, not 500
@end

@case
id: FE-0337
surface: /admin/equipment/[id]
catégorie: auth-tenant
tag: back
précondition: non-admin (or Bearer PAT, even an admin's)
action: PATCH /api/admin/equipment/:id
attendu: AdminUser rejects PAT first (TokenAuth extension → Forbidden) and non-admins (Some(_) → Forbidden) → 403; anonymous → 401
@end

@case
id: FE-0338
surface: /admin/equipment/[id]
catégorie: intégrité-données
tag: back
précondition: item is referenced by photos via photo_filters and the rename changes display_name
action: PATCH /api/admin/equipment/:id with a new model
attendu: if display_name != row.display_name → photos::filters_cache::rebuild_for_item(&mut tx, id) runs in the same tx so the denormalized photos.filters cache stays consistent
@end

@case
id: FE-0339
surface: /admin/equipment/[id]
catégorie: contrat-API
tag: back
précondition: admin submits specs whose discriminator kind mismatches the item's kind
action: PATCH /api/admin/equipment/:id (item kind=telescope) with specs {kind:"camera",...}
attendu: specs::ensure_matches_kind validates before any write → AppError::Validation → 422, clean rollback
@end

@case
id: FE-0340
surface: /admin/equipment/[id]
catégorie: frontend
tag: front
précondition: item has usage_count>0 or setup_count>0
action: open editor
attendu: inUse = usage_count>0 || Number(setup_count)>0 disables the Delete button (title "In use — cannot delete") and shows the hint; save still allowed
@end

@case
id: FE-0341
surface: /admin/equipment/[id]
catégorie: sécurité
tag: back
précondition: admin DELETEs an orphan item that has 0 usage, 0 setups, 0 filters
action: DELETE /api/admin/equipment/:id
attendu: delete() pre-check (usage_count, setup_items, photo_filters all 0) passes → row deleted, specs sub-table cascades; a still-referenced item → AppError::Conflict("item is still in use...") → 409 (mirrors ON DELETE RESTRICT)
@end

@case
id: FE-0342
surface: /admin/settings
catégorie: entrées-limites
tag: back
précondition: admin sets free tier limit above the cap
action: PUT /api/admin/settings with free_upload_max_mb=200000
attendu: settings::update bound check (MIN_UPLOAD_MB..=MAX_UPLOAD_MB i.e. 1..=100000) → AppError::Validation("upload limit must be 1..=100000 MB") → 422
@end

@case
id: FE-0343
surface: /admin/settings
catégorie: états
tag: back
précondition: app_settings row absent (fresh DB / before first save)
action: GET /api/admin/settings
attendu: settings::get fetch_optional → Ok(None) logs warn "app_settings row missing" and returns AppSettings::default() (signups_enabled=true, 50/200 MB) — reader is fail-safe, never 500
@end

@case
id: FE-0344
surface: /admin/settings
catégorie: concurrence
tag: back
précondition: two admins load settings, both edit, both save
action: admin A PUT free=80, admin B PUT free=30 moments later
attendu: settings::update is an unconditional upsert on id=1 (on conflict do update); no version guard → silent last-write-wins (B's 30 overwrites A's 80); updated_by stamps only the last writer
@end

@case
id: FE-0345
surface: /admin/settings
catégorie: auth-tenant
tag: back
précondition: authenticated non-admin
action: PUT /api/admin/settings
attendu: AdminUser(admin) extractor → Some(_) non-admin → AppError::Forbidden → 403; PAT auth also rejected before is_admin check
@end

@case
id: FE-0346
surface: /admin/settings
catégorie: intégrité-données
tag: back
précondition: app_settings is a singleton (id=1)
action: PUT /api/admin/settings repeatedly
attendu: insert into app_settings (id, ...) values (1, ...) on conflict (id) do update — only ever one row; updated_at=now() and updated_by stamped each write
@end

@case
id: FE-0347
surface: /admin/settings
catégorie: contrat-API
tag: back
précondition: admin PUTs a partial body missing subscriber_upload_max_mb
action: PUT /api/admin/settings with {signups_enabled, free_upload_max_mb}
attendu: AppSettings has no serde defaults → Json extractor fails to deserialize → 422 (all three fields required, replace-all semantics)
@end

@case
id: FE-0348
surface: /admin/settings
catégorie: frontend
tag: front
précondition: updateSettings throws (non-ok response)
action: click Save settings
attendu: save() catch sets errorMsg=(e).message rendered in .err; busy reset in finally; draft seeded once via untrack so a failed save doesn't desync the form
@end

@case
id: FE-0349
surface: /admin/settings
catégorie: sécurité
tag: front
précondition: non-admin navigates directly to /admin/settings
action: GET /admin/settings
attendu: /admin/+layout.server.ts: !locals.user.isAdmin → redirect(303, '/') (no leak that /admin exists); backend AdminUser is the real boundary on the PUT
@end

@case
id: FE-0350
surface: /account/frames
catégorie: entrées-limites
tag: front
précondition: authenticated user
action: GET /account/frames?filter=bogus&sort=weird&view=xyz
attendu: load casts via `as` without validation; filter !== 'drafts'/'published' falls through the ternary to the merged [...drafts, ...published] list; sort !== 'newest' uses dir=1 (oldest) — no crash, defaults applied loosely
@end

@case
id: FE-0351
surface: /account/frames
catégorie: états
tag: front
précondition: user has zero published photos and zero drafts
action: GET /account/frames
attendu: counts.all === 0 → isEmpty true → renders the empty plate hero with "Upload a frame" CTA, no PhotosTable
@end

@na
surface: /account/frames
catégorie: concurrence
raison: /account/frames is read-only — load only fires three GETs (/api/me/stats, /api/photos?owner_id, /api/photos?drafts=true); it performs no mutation, so there is no concurrent-writer race on this surface.
@end

@case
id: FE-0353
surface: /account/frames
catégorie: auth-tenant
tag: front
précondition: no session
action: GET /account/frames?filter=drafts
attendu: load: if (!locals.user) redirect(303, `/signin?next=${encodeURIComponent(url.pathname + url.search)}`) — preserves the query string for post-login return
@end

@case
id: FE-0354
surface: /account/frames
catégorie: intégrité-données
tag: front
précondition: backend returns appreciation_count as a bigint/string
action: load normalises each photo row
attendu: normalisePhoto spreads the row and coerces appreciation_count = Number(rec.appreciation_count ?? 0); stats fields likewise Number()-cast so the page never renders NaN
@end

@case
id: FE-0355
surface: /account/frames
catégorie: contrat-API
tag: front
précondition: /api/me/stats returns a non-ok status (AppError json or gateway body)
action: GET /account/frames
attendu: fetchJson: if (!r.ok) error(502, 'Backend error') — real error page instead of NaN stats or an opaque r.json() reject
@end

@case
id: FE-0356
surface: /account/frames
catégorie: frontend
tag: front
précondition: merged list spans drafts and published with mixed created_at
action: GET /account/frames?sort=oldest
attendu: rows sorted by created_at.localeCompare with dir (newest=-1, oldest=1) across the concatenated list — a plain reverse() would only flip concat order, this sorts by timestamp; PhotosTable receives coherent ordering
@end

@case
id: FE-0357
surface: /account/frames
catégorie: sécurité
tag: back
précondition: attacker edits the request to request another user's drafts
action: GET /api/photos?drafts=true&owner_id=<victim uuid>
attendu: photos::list::handler drafts branch: if requested != me.id → AppError::Forbidden → 403; the published owner_id path (list_by_owner) only returns published_at is not null rows, so no private leak there either
@end

<!-- ===== batch D ===== -->

@case
id: FE-0400
surface: /upload
catégorie: entrées-limites
tag: front
précondition: free-tier user (TIER_MAX = 50 MiB in +page.svelte:21) drops a 60 MiB JPEG
action: onFiles() compares file.size > TIER_MAX and sets showUpgrade=true, skipping the slot
attendu: TierUpgradeModal opens; no slot pushed; no POST /api/uploads/init fired for that file
@end

@case
id: FE-0401
surface: /upload
catégorie: entrées-limites
tag: back
précondition: free tier; settings.upload_max_bytes("free") returns the 50 MiB default; client TIER_MAX bypassed (forged init)
action: POST /api/uploads/init with files[0].size = 60*1024*1024
attendu: upload_init.rs:59 returns AppError::QuotaExceeded("file ... exceeds {max_bytes} bytes")
@end

@case
id: FE-0402
surface: /upload
catégorie: entrées-limites
tag: back
précondition: authenticated; platesolve client configured
action: POST /api/uploads/init with files[0].mime = "image/gif"
attendu: upload_init.rs:84 returns AppError::UnsupportedFormat("image/gif")
@end

@case
id: FE-0403
surface: /upload
catégorie: entrées-limites
tag: back
précondition: 13 files dragged onto UploadDropzone
action: onFiles() truncates to MAX_QUEUE=12 (slice(0,room)) and sets queueCapWarning; if forged, POST /api/uploads/init with 13 files
attendu: front: queueCapWarning "Queue caps at 12 files"; back: upload_init.rs:42 AppError::Validation("files must be 1..=12")
@end

@case
id: FE-0404
surface: /upload
catégorie: états
tag: back
précondition: pending photo row exists; presigned PUT to originals/<id> has not yet completed
action: POST /api/uploads/:id/finalize before the S3 object lands
attendu: upload_finalize.rs:50 state.storage.get returns None → AppError::PendingFinalizeStuck("no object at storage_key — did the PUT succeed?")
@end

@case
id: FE-0405
surface: /upload
catégorie: états
tag: back
précondition: XISF upload; platesolve configured; PUT completed
action: POST /api/uploads/:id/finalize for mime application/x-xisf
attendu: upload_finalize.rs:88 mark_awaiting_calibration; FinalizeResp.status = "awaiting-calibration", display_key = None; auto_calibrate_xisf spawned
@end

@case
id: FE-0406
surface: /upload
catégorie: concurrence
tag: back
précondition: two browser tabs both call finalize on the same pending photo id
action: both issue POST /api/uploads/:id/finalize concurrently
attendu: upload_finalize.rs:68 UPDATE ... where status in ('pending','failed') claims for one; the loser gets claimed==0 → AppError::Conflict("finalize already in progress")
@end

@case
id: FE-0407
surface: /upload
catégorie: concurrence
tag: back
précondition: init pre-check (upload_init.rs:93) missed a concurrent duplicate of the same owner+hash
action: two POST /api/uploads/init race inserting the same original_hash
attendu: photos_owner_hash_uidx constraint fires (upload_init.rs:140) → AppError::Conflict("file already uploaded")
@end

@case
id: FE-0408
surface: /upload
catégorie: auth-tenant
tag: back
précondition: unauthenticated client (no session cookie)
action: POST /api/uploads/init
attendu: CurrentUser extractor rejects before handler runs → 401 (no photos row inserted)
@end

@case
id: FE-0409
surface: /upload
catégorie: auth-tenant
tag: back
précondition: user B holds user A's pending photo uuid
action: user B calls POST /api/uploads/:id/finalize on A's id
attendu: upload_finalize.rs:36 row.owner_id != user.id → AppError::NotFound("photo") (existence-hiding, NOT Forbidden)
@end

@case
id: FE-0410
surface: /upload
catégorie: intégrité-données
tag: back
précondition: init batch of 12 files where files[5] duplicates an existing owner+hash
action: POST /api/uploads/init with the 12-file batch
attendu: upload_init.rs:100 returns AppError::Conflict on the dup; tx never commits (upload_init.rs:160) so files 0..11 get NO photos rows — whole-batch rollback
@end

@case
id: FE-0411
surface: /upload
catégorie: intégrité-données
tag: back
précondition: client PUT body length differs from the size sent to init (e.g. retried with a re-encoded file)
action: PUT to presigned_put URL signed with content_length(body_bytes) (s3.rs:196) using a different byte count
attendu: S3 rejects with SignatureDoesNotMatch; finalize later sees no object → AppError::PendingFinalizeStuck
@end

@case
id: FE-0412
surface: /upload
catégorie: contrat-API
tag: back
précondition: valid 1-file init
action: POST /api/uploads/init
attendu: InitResponse { files: [InitFile{ photo_id, short_id, presigned_put_url }] } (upload_init.rs:23-33)
@end

@case
id: FE-0413
surface: /upload
catégorie: frontend
tag: front
précondition: a slot is mid-upload past 50% (progress.state='uploading', pct>50)
action: user clicks cancel on UploadFileRow → cancelSlot()
attendu: +page.svelte:172 opens cancelSlotOpen ConfirmDialog ("X% complete will be lost") before performCancelSlot fires DELETE /api/uploads/:id
@end

@case
id: FE-0414
surface: /upload
catégorie: sécurité
tag: back
précondition: attacker uploads a PHP/HTML payload renamed to .jpg with mime image/jpeg
action: PUT the file, then POST /api/uploads/:id/finalize
attendu: upload_finalize.rs:57 magic::matches_mime fails → mark_failed("magic-byte mismatch") + AppError::MagicByteMismatch
@end

@case
id: FE-0415
surface: /upload
catégorie: sécurité
tag: front
précondition: a finalized JPEG carried embedded GPS EXIF
action: pipeline::finalize extracts exif_json (upload_finalize.rs:103) and stores it
attendu: exif_json persisted on photos; ra_deg/dec_deg may leak the shooting site — verify exif scrub/whitelist in photos/exif.rs before public display
@end

@case
id: FE-0416
surface: /upload/[id]/verify
catégorie: entrées-limites
tag: back
précondition: verify form save with a 2100-char caption
action: form action ?/publish → PUT /api/photos/:id with caption length 2100
attendu: metadata.rs:155 MAX_CAPTION_CHARS=2000 → AppError::Validation("caption too long (max 2000 chars)")
@end

@case
id: FE-0417
surface: /upload/[id]/verify
catégorie: entrées-limites
tag: back
précondition: verify form save with 9 tags
action: PUT /api/photos/:id with tags array of length 9
attendu: metadata.rs:139 → AppError::Validation("max 8 tags")
@end

@case
id: FE-0418
surface: /upload/[id]/verify
catégorie: états
tag: back
précondition: XISF photo still status='awaiting-calibration' (display master not yet returned by platesolve)
action: form action ?/publish → POST /api/photos/:id/publish
attendu: publish.rs:31 status != "ready" → AppError::bad_request("photo not ready: pipeline still processing or failed")
@end

@case
id: FE-0419
surface: /upload/[id]/verify
catégorie: états
tag: front
précondition: standard JPEG photo whose thumbnails are still decoding (status='processing')
action: open /upload/<id>/verify; background poll runs (verify +page.svelte:349)
attendu: VerifyPane shows processing state and the publish action stays gated until status flips to 'ready'
@end

@case
id: FE-0420
surface: /upload/[id]/verify
catégorie: concurrence
tag: back
précondition: verify autosave PUT (filter_item_ids → filters_cache::rebuild, no row lock) races a POST /api/photos/:id/apply-setup (apply_setup.rs:73 select ... for update) on the same photo
action: fire PUT /api/photos/:id and apply-setup concurrently, both rewriting photo_filters
attendu: apply_setup holds the row lock; metadata.rs:369 delete+insert path can interleave around it — last committer wins photo_filters; both call filters_cache::rebuild so photos.filters cache stays consistent with whichever junction state commits last
@end

@case
id: FE-0421
surface: /upload/[id]/verify
catégorie: concurrence
tag: front
précondition: two autosave PUTs queued as the user edits quickly
action: verify +page.svelte:461 aborts the prior autosaveAbort controller before issuing the next fetch
attendu: only the latest PUT /api/photos/:id survives; the superseded request is AbortController-cancelled, preventing an older patch from clobbering newer field values
@end

@case
id: FE-0422
surface: /upload/[id]/verify
catégorie: auth-tenant
tag: back
précondition: user B opens /upload/<A's id>/verify
action: load() fetches GET /api/photos/:id then checks photo.owner_id !== locals.user.id (verify +page.server.ts:20)
attendu: error(404, 'Not found') from the load; and a forged PUT /api/photos/:id hits metadata.rs:111 → AppError::Forbidden
@end

@case
id: FE-0423
surface: /upload/[id]/verify
catégorie: auth-tenant
tag: back
précondition: user B applies user A's setup uuid to B's own photo
action: POST /api/photos/:id/apply-setup with A's setup_id
attendu: apply_setup.rs:63 setup where owner_id=$2 fails → AppError::NotFound("setup not found")
@end

@case
id: FE-0424
surface: /upload/[id]/verify
catégorie: intégrité-données
tag: back
précondition: filter_item_ids contains a duplicate id and one non-filter equipment_item id
action: PUT /api/photos/:id with filter_item_ids = [f1, f1, camera_id]
attendu: metadata.rs:344 dedups f1; metadata.rs:364 count mismatch (camera_id kind != 'filter') → AppError::Validation("filter_item_ids contains an unknown id or a non-filter kind"); junction unchanged (tx rollback)
@end

@case
id: FE-0425
surface: /upload/[id]/verify
catégorie: intégrité-données
tag: back
précondition: photo already plate-solved (platesolve_pixel_scale_arcsec not null); user applies a setup whose scope derives focal_mm/aperture_f
action: POST /api/photos/:id/apply-setup
attendu: apply_setup.rs:218 already_solved drops derived_focal_mm/derived_aperture_f to None so the solve's measured framing is never clobbered
@end

@case
id: FE-0426
surface: /upload/[id]/verify
catégorie: contrat-API
tag: back
précondition: XISF re-solve issued with a large body through the SvelteKit /api/* proxy
action: POST /api/photos/:id/platesolve (verify +page.svelte:111) carrying a body above adapter-node BODY_SIZE_LIMIT
attendu: proxy +server.ts kills the stream → SvelteKit 500 {"message":"Internal Error"} (capital-I, no error field) — distinct from a backend AppError envelope {"error":...,"message":"internal error"}
@end

@case
id: FE-0427
surface: /upload/[id]/verify
catégorie: contrat-API
tag: back
précondition: publish action persists the form then flips published_at
action: form action ?/publish runs callPut PUT /api/photos/:id then POST /api/photos/:id/publish (verify +page.server.ts:243)
attendu: on PUT failure return fail(status); only on PUT ok does publish fire → redirect(303, /photo/:id); publish.rs sets published_at=now(), last_step='caption'
@end

@case
id: FE-0428
surface: /upload/[id]/verify
catégorie: frontend
tag: front
précondition: load() best-effort fetch of GET /api/photos/:id/platesolve-status throws
action: open /upload/<id>/verify
attendu: verify +page.server.ts:89 catch leaves platesolveStatus = null; the plate-solve panel renders the idle state rather than erroring the page
@end

@case
id: FE-0429
surface: /upload/[id]/verify
catégorie: sécurité
tag: back
précondition: legacy cache-string tokens in photos.filters that have no matching structured filter_item
action: load() computes orphans = cacheTokens not in known display_names (verify +page.server.ts:77)
attendu: orphans rendered as read-only "legacy" chips; a verify save sending filter_item_ids replaces the junction and filters_cache::rebuild overwrites the cache, dropping orphan tokens (no silent re-injection)
@end

@case
id: FE-0430
surface: /upload/batch
catégorie: entrées-limites
tag: back
précondition: batch apply with target submitted as empty string
action: POST /api/photos/batch/apply with target=""
attendu: batch_apply.rs:21 → AppError::bad_request("target cannot be empty string")
@end

@case
id: FE-0431
surface: /upload/batch
catégorie: entrées-limites
tag: back
précondition: batch apply over 51 ids
action: POST /api/photos/batch/apply with 51 ids
attendu: batch_apply.rs:18 → AppError::bad_request("too many ids")
@end

@case
id: FE-0432
surface: /upload/batch
catégorie: états
tag: back
précondition: one of the batch ids is already published (published_at not null)
action: POST /api/photos/batch/apply over a mix of draft + published ids
attendu: batch_apply.rs:47 → AppError::bad_request("one or more ids refer to published photos") (whole batch rejected, no partial write)
@end

@case
id: FE-0433
surface: /upload/batch
catégorie: concurrence
tag: back
précondition: batch apply target + tags both set; tags path deletes then re-attaches
action: POST /api/photos/batch/apply with target and tags
attendu: batch_apply.rs:73 runs delete photo_tags + tags::attach inside one tx so a mid-attach failure rolls back rather than stripping tags from all ids
@end

@case
id: FE-0434
surface: /upload/batch
catégorie: auth-tenant
tag: back
précondition: batch includes a photo id owned by another user
action: load() runs getPhoto per id then checks p.owner_id !== locals.user.id (batch/+page.server.ts:28)
attendu: batch/+page.server.ts:28 error(403,'not owner') in load; forged POST /api/photos/batch/apply hits batch_apply.rs:43 → AppError::Forbidden
@end

@case
id: FE-0435
surface: /upload/batch
catégorie: intégrité-données
tag: back
précondition: batch apply target sets the free-text target across N photos
action: POST /api/photos/batch/apply with target="M31"
attendu: batch_apply.rs:64 loops attach_primary_by_freetext per id inside one tx; photos.target column and the target join rows commit atomically together
@end

@case
id: FE-0436
surface: /upload/batch
catégorie: contrat-API
tag: back
précondition: one batch id does not exist
action: POST /api/photos/batch/apply where owners.len() != ids.len()
attendu: batch_apply.rs:37 → AppError::not_found("one or more photo ids do not exist")
@end

@case
id: FE-0437
surface: /upload/batch
catégorie: frontend
tag: front
précondition: /upload/batch?ids=<single id>
action: load() sees ids.length === 1
attendu: batch/+page.server.ts:12 redirect(303, /upload/<id>/verify) — single-frame batch collapses to the verify page
@end

@case
id: FE-0438
surface: /upload/batch
catégorie: sécurité
tag: front
précondition: attacker crafts /upload/batch?ids=<own>,<victim>
action: load() Promise.all getPhoto over both ids
attendu: getPhoto for the victim id returns 404 (owner-gated backend) so Promise.all rejects, OR owner check error(403); no victim metadata reaches the page
@end

@case
id: FE-0439
surface: /upload/batch/edit
catégorie: entrées-limites
tag: back
précondition: ?ids= present but empty after split/filter
action: GET /upload/batch/edit?ids=,,,
attendu: batch/edit/+page.server.ts:11 error(400,'no ids')
@end

@case
id: FE-0440
surface: /upload/batch/edit
catégorie: états
tag: back
précondition: batch of 3 where one is 'failed', one 'processing', one 'ready'
action: form action ?/publish_all → POST /api/photos/batch/publish
attendu: batch_publish.rs builds skipped[] with SkipReason::Failed + SkipReason::StillProcessing and published[] for the ready one; redirect carries ?published=1&skipped=2
@end

@case
id: FE-0441
surface: /upload/batch/edit
catégorie: concurrence
tag: back
précondition: same batch published from two tabs
action: both fire POST /api/photos/batch/publish
attendu: batch_publish.rs:45 second pass sees published_at set → SkipReason::AlreadyPublished (no double publish; not an error)
@end

@case
id: FE-0442
surface: /upload/batch/edit
catégorie: auth-tenant
tag: back
précondition: ?ids= includes a victim photo id
action: load() getPhoto per id; backend 404/403 caught
attendu: batch/edit/+page.server.ts:25 re-emits ApiError 404/403 as error(404,'Not found'); owner check at :32 also error(404)
@end

@case
id: FE-0443
surface: /upload/batch/edit
catégorie: intégrité-données
tag: back
précondition: batch publish where one id was deleted between load and submit
action: POST /api/photos/batch/publish with that id
attendu: batch_publish.rs:31 rows.len() != ids.len() → AppError::not_found("one or more photo ids do not exist"); tx never commits so no partial publish
@end

@case
id: FE-0444
surface: /upload/batch/edit
catégorie: contrat-API
tag: back
précondition: successful publish_all of 3 ready photos
action: POST /api/photos/batch/publish
attendu: BatchPublishResponse { published:[PublishedItem{id,short_id}], skipped:[] }; load redirects to /account/frames?published=3&skipped=0 (batch/edit/+page.server.ts:64)
@end

@case
id: FE-0445
surface: /upload/batch/edit
catégorie: frontend
tag: front
précondition: ?selected= points at an id not in the ids list
action: load() resolves selected
attendu: batch/edit/+page.server.ts:36 falls back to ids[0]; BatchRibbon highlights the first frame instead of erroring
@end

@case
id: FE-0446
surface: /upload/batch/edit
catégorie: sécurité
tag: front
précondition: VerifyPane autosave runs per selected frame with autosave={true}
action: editing a frame triggers PUT /api/photos/:id (filter_item_ids → filters_cache::rebuild)
attendu: each autosave is owner-scoped via metadata.rs:111 Forbidden check; a switched ?selected to a forged id still fails the per-id PUT owner gate
@end

@case
id: FE-0447
surface: /me/drafts
catégorie: entrées-limites
tag: back
précondition: cursor query param is not RFC3339
action: GET /api/photos/me/drafts?cursor=notadate
attendu: drafts_list.rs:30 DateTime::parse_from_rfc3339 fails → AppError::Validation("bad cursor")
@end

@case
id: FE-0448
surface: /me/drafts
catégorie: entrées-limites
tag: back
précondition: caller requests limit=999
action: GET /api/photos/me/drafts?limit=999
attendu: drafts_list.rs:24 clamp(1,50) caps the SQL LIMIT at 50
@end

@case
id: FE-0449
surface: /me/drafts
catégorie: états
tag: front
précondition: a draft row is still status='processing' / display_key null
action: load lists it; DraftTile renders thumb_url
attendu: drafts_list.rs:64 thumb_url = {cdn}/img/<id>?w=320 has no display/<id>.jpg yet → broken/placeholder thumbnail until pipeline writes the display master
@end

@na
surface: /me/drafts
catégorie: concurrence
raison: drafts_list.rs:35 is a single read-only SELECT (no INSERT/UPDATE/DELETE, no tx); there is no write to race, so no concurrency invariant exists on this surface.
@end

@case
id: FE-0450
surface: /me/drafts
catégorie: auth-tenant
tag: back
précondition: any authenticated user
action: GET /api/photos/me/drafts
attendu: drafts_list.rs:39 where owner_id = $1 (user.id) scopes the list to the caller only; another user's drafts are never returned
@end

@case
id: FE-0451
surface: /me/drafts
catégorie: intégrité-données
tag: back
précondition: a published photo exists for the owner
action: GET /api/photos/me/drafts
attendu: drafts_list.rs:40 where published_at is null excludes it — drafts list and published feed never overlap
@end

@case
id: FE-0452
surface: /me/drafts
catégorie: contrat-API
tag: back
précondition: exactly `limit` rows returned (full page)
action: GET /api/photos/me/drafts?limit=24 with 24 matching rows
attendu: drafts_list.rs:68 next_cursor = last row created_at.to_rfc3339(); fewer than limit → next_cursor null (DraftListResponse{items,next_cursor})
@end

@case
id: FE-0453
surface: /me/drafts
catégorie: frontend
tag: front
précondition: owner has zero drafts
action: open /me/drafts
attendu: drafts +page.svelte:38 renders EmptyState ("No drafts yet", ctaHref="/upload"); Resume recent button hidden
@end

@case
id: FE-0454
surface: /me/drafts
catégorie: sécurité
tag: front
précondition: user clicks Resume recent with a mix of drafts older/newer than the 1h window
action: drafts +page.svelte:11 resumeRecent filters to ids within newest-60min cutoff
attendu: only ids the load already owner-scoped go into /upload/batch/edit?ids=...; no foreign id can be injected since the list itself is owner-filtered by drafts_list.rs:39
@end

<!-- ===== batch E ===== -->

@case
id: FE-0500
surface: /explore
catégorie: entrées-limites
tag: back
précondition: explore.rs maps q.since via match; only "24h"/"7d"/"30d"/"all"/None are valid.
action: GET /api/explore?since=garbage
attendu: match arm Some(_) returns Err(AppError::bad_request("since_invalid")) → 400, not a silent fallback to 7d.
@end

@case
id: FE-0501
surface: /explore
catégorie: entrées-limites
tag: back
précondition: explore.rs computes limit = q.limit.unwrap_or(24).clamp(1, MAX_LIMIT) with MAX_LIMIT=60.
action: GET /api/explore?limit=100000 and GET /api/explore?limit=-5
attendu: limit clamped to 60 (upper) and 1 (lower) respectively; SQL `limit $7` receives limit+1, never the raw user value.
@end

@case
id: FE-0502
surface: /explore
catégorie: entrées-limites
tag: back
précondition: +page.server.ts casts `sort` to 'newest' | 'most-appreciated' without validation; explore.rs `match sort` falls through `_` to the newest branch.
action: GET /api/explore?sort=bogus
attendu: 200 with the newest-sorted query (the `_ =>` arm runs); next_cursor encodes appreciations: None.
@end

@case
id: FE-0503
surface: /explore
catégorie: états
tag: back
précondition: explore.rs default `since` is 7d (Some("7d") via +page.server.ts default), filtering published_at > now() - 7 days.
action: load /explore with all photos older than 7 days, then load /explore?since=all
attendu: first grid is empty (the 7d window, not an SSR bug); ?since=all sets since_seconds=None so the published_at interval filter is dropped and tiles appear.
@end

@case
id: FE-0504
surface: /explore
catégorie: états
tag: front
précondition: +page.server.ts wraps fetchExplore in try/catch and throws error(500, 'Failed to load explore feed') on any reject; +error.svelte renders it.
action: backend /api/explore returns 500/unreachable during SSR
attendu: SvelteKit renders the 500 error page with 'Failed to load explore feed', not a blank grid; the parallel statsPromise still resolves null without aborting.
@end

@case
id: FE-0505
surface: /explore
catégorie: concurrence
tag: back
précondition: cursor.rs encodes (published_at, id) and newest query uses `(p.published_at, p.id) < ($1, $2)` strict tuple comparison.
action: new photos are published after the SSR page seeds cursor, then loadMoreFn() fires fetchExplore with the stored cursor
attendu: keyset pagination returns only rows strictly older than the cursor tuple; freshly published rows above the cursor are not duplicated into the next page.
@end

@case
id: FE-0506
surface: /explore
catégorie: concurrence
tag: back
précondition: DiscoveryHeader photoCount={data.totalFrames ?? data.initial.photos.length}; totalFrames comes from /api/site/stats (frames), independent of the feed query.
action: photos are published between the stats fetch and the feed fetch (both in Promise.all)
attendu: the eyebrow frame count (site/stats.frames) can drift from the number of rendered tiles; count is a site-wide total, not the filtered feed length.
@end

@case
id: FE-0507
surface: /explore
catégorie: auth-tenant
tag: back
précondition: explore.rs: (true, None) on following arm returns Json(DiscoveryPage{ photos: vec![], next_cursor: None }).
action: anonymous (no session) GET /api/explore?following=true
attendu: 200 empty page, not 401; an anonymous caller follows nobody so the follows EXISTS subquery is skipped via the early return.
@end

@case
id: FE-0508
surface: /explore
catégorie: auth-tenant
tag: back
précondition: both explore.rs queries gate on `p.published_at is not null and p.status = 'ready'`; drafts are published_at IS NULL (photos/get.rs is_draft).
action: owner has draft (unpublished) and `processing`-status photos; any user GETs /api/explore
attendu: neither draft nor processing rows appear — the WHERE clause excludes published_at IS NULL and status<>'ready'; no per-viewer leak.
@end

@case
id: FE-0509
surface: /explore
catégorie: intégrité-données
tag: back
précondition: explore.rs orders most-appreciated by appreciations_count desc using the denormalized p.appreciations_count column (cursor carries cur_apps).
action: GET /api/explore?sort=most-appreciated where p.appreciations_count is stale vs actual appreciation rows
attendu: ordering and cursor tiebreak follow the cached appreciations_count column verbatim; the feed trusts the denormalized counter, not a live COUNT.
@end

@case
id: FE-0510
surface: /explore
catégorie: contrat-API
tag: back
précondition: cursor::decode returns Err(AppError::bad_request("cursor_invalid")) on non-base64 or non-Cursor-JSON; explore.rs does `.map(cursor::decode).transpose()?`.
action: GET /api/explore?cursor=not-base64!!
attendu: 400 with error "cursor_invalid"; the `?` propagates before any SQL runs.
@end

@case
id: FE-0511
surface: /explore
catégorie: frontend
tag: front
précondition: +page.svelte uses {#key `${sort}|${since}|${category ?? ''}|${following}`} around CrossAuthorGrid and an $effect that resets cursor = data.initial.next_cursor on data change.
action: applyFilter changes sort/since via goto(replaceState) so data reloads
attendu: the #key block tears down and rebuilds CrossAuthorGrid for the new filter set and the $effect reseeds `cursor`, so loadMoreFn closes over the new filter's next_cursor (no stale cursor from the prior filter).
@end

@case
id: FE-0512
surface: /explore
catégorie: sécurité
tag: front
précondition: +page.svelte builds canonicalUrl from page.url.origin and ogImage from cdn(first.id,{w:1200}); pageTitle uses categoryLabel(data.category) not raw category.
action: GET /explore?category=<script>alert(1)</script>
attendu: backend `($4::text is null or p.category = $4)` matches no category enum value → empty feed; the title path runs categoryLabel(), and category is never reflected unescaped into a <script> or attribute, so no XSS.
@end

@case
id: FE-0513
surface: /search
catégorie: entrées-limites
tag: back
précondition: search.rs trims q.q; if term.is_empty() returns Err(AppError::bad_request("q_empty")). Frontend +page.server.ts short-circuits `if (!q.trim()) return { q, initial: null }` so the backend is never called for empty q.
action: direct GET /api/search?q=%20%20 (whitespace only)
attendu: backend returns 400 "q_empty"; via the page, an empty/whitespace q renders the empty-state copy without any API call.
@end

@case
id: FE-0514
surface: /search
catégorie: entrées-limites
tag: back
précondition: search.rs caps results at TARGET_CAP=5, USER_CAP=5, PHOTO_CAP=24 via SQL `limit $2`; there is no offset/page param.
action: GET /api/search?q=<very long 5000-char string>
attendu: pattern = format!("%{}%", term.to_lowercase()) is passed as a bound parameter to ILIKE/LIKE; query runs and returns at most 5+5+24 rows; no pagination param exists to abuse.
@end

@case
id: FE-0515
surface: /search
catégorie: états
tag: front
précondition: +page.svelte computes totalCount = targets.length+users.length+photos.length and renders <p class="no-results">No results found for "{data.q}".</p> when totalCount===0.
action: search a term that matches nothing, e.g. /search?q=zzzqqq
attendu: backend returns SearchResults with empty targets/users/photos arrays (200, not 404); UI shows the no-results paragraph reflecting data.q.
@end

@na
surface: /search
catégorie: concurrence
raison: search.rs is a single stateless snapshot query with no cursor/pagination and PHOTO_CAP=24 (no next_cursor field in SearchResults); there is no multi-page feed to drift, so no concurrency window exists.
@end

@na
surface: /search
catégorie: auth-tenant
raison: search.rs `get` takes only State and Query<Q> (no OptionalUser/AuthUser extractor) and all three subqueries hard-filter `p.published_at is not null and p.status='ready'` / public users; results are identical for every caller and drafts cannot leak, so there is no tenant dimension.
@end

@case
id: FE-0517
surface: /search
catégorie: intégrité-données
tag: back
précondition: target hit photo_count is a correlated subquery `count(*) ... where pt.target_id=t.id and p.published_at is not null and p.status='ready'`, recomputed live (not a cached column).
action: GET /api/search?q=M31 immediately after a photo for that target is unpublished/deleted
attendu: SearchTargetHit.photo_count reflects the live join count at query time; a just-removed photo is not counted, so the badge matches actual visible rows.
@end

@case
id: FE-0518
surface: /search
catégorie: contrat-API
tag: back
précondition: Q has a single required field `pub q: String` (not Option); axum Query rejects a missing `q` with a 400 deserialization error.
action: GET /api/search with no q param
attendu: axum Query<Q> fails to deserialize the missing required field → 400 before handler body (distinct from the q_empty 400 produced inside the handler for blank q).
@end

@case
id: FE-0519
surface: /search
catégorie: frontend
tag: front
précondition: +page.svelte sets <meta name="robots" content="noindex, follow" /> and reflects {data.q} in <title> and pageDescription via Svelte text interpolation (auto-escaped).
action: SSR-render /search?q=Andromeda
attendu: page is noindex (so infinite query permutations don't pollute the index) but follow; data.q appears in title/description through Svelte's escaped interpolation.
@end

@case
id: FE-0520
surface: /search
catégorie: sécurité
tag: back
précondition: search.rs binds `pattern` to ILIKE/LIKE as a parameter ($1) — never string-concatenated SQL; frontend interpolates data.q only in escaped text nodes and no-results copy.
action: GET /api/search?q=' OR 1=1 -- and /search?q=<img src=x onerror=alert(1)>
attendu: the quote/HTML is treated as a literal ILIKE substring (parameterized, no SQL injection) and Svelte escapes it in "No results found for "..."" so no XSS; only published/ready public rows can ever match (no enumeration of drafts).
@end

@case
id: FE-0521
surface: /photographers
catégorie: entrées-limites
tag: back
précondition: photographer_index.rs limit = q.limit.unwrap_or(24).clamp(1,60); +page.server.ts hardcodes limit=24 but the API accepts q.limit.
action: GET /api/photographers?limit=99999 and ?limit=0
attendu: limit clamped to 60 and 1 respectively; SQL `limit $3` uses the clamped value.
@end

@case
id: FE-0522
surface: /photographers
catégorie: entrées-limites
tag: front
précondition: +page.server.ts: sort = VALID_SORTS.has(...) ? cast : 'active', VALID_SORTS={active,followers,recent}.
action: GET /photographers?sort=banana
attendu: frontend coerces to 'active' before building `${API}/api/photographers?sort=active&limit=24`; backend `match sort` also routes unknown to the `_ =>` active branch, so the result is the active (frame_count) ordering.
@end

@case
id: FE-0523
surface: /photographers
catégorie: états
tag: front
précondition: +page.server.ts: `let initial = { items: [], next_cursor: null }; if (r.ok) initial = ...`. Handler filters `where frame_count > 0`.
action: backend returns non-2xx, or no user has any published photo
attendu: page still renders with initial.items === [] (no thrown error); only users with frame_count>0 ever appear, so accounts with zero published photos are absent.
@end

@case
id: FE-0524
surface: /photographers
catégorie: concurrence
tag: back
précondition: active/followers cursors use CountCursor{count,id} with `(count = $1 and id > $2)` ascending-id tiebreak; recent uses DateCursor with `created_at < $1 or (created_at=$1 and id>$2)`.
action: a photographer's frame_count changes (new upload) between page 1 and the cursor'd page 2 of sort=active
attendu: keyset uses the stale count captured in the cursor; a row whose count moved above the cursor boundary may be skipped or repeated, but the id tiebreak keeps ordering deterministic within equal counts.
@end

@case
id: FE-0525
surface: /photographers
catégorie: auth-tenant
tag: back
précondition: list() takes only State and Query<ListQ>; integration_seconds = sum(p.exposure_s*coalesce(p.sessions,1)) filter (where p.published_at is not null), frame_count counts only published photos.
action: any caller (anon or authed) GET /api/photographers
attendu: identical output regardless of session; draft photos are excluded from frame_count and integration_seconds via the `filter (where p.published_at is not null)` clause, so unpublished work never inflates a public profile stat.
@end

@case
id: FE-0526
surface: /photographers
catégorie: intégrité-données
tag: back
précondition: follower_count = count(distinct f.follower_id) and frame_count = count(distinct p.id) computed in the same `stats` CTE via LEFT JOINs to photos and follows.
action: a user with many photos AND many followers is aggregated
attendu: distinct counts prevent the photos×follows cartesian fan-out from multiplying either counter; frame_count and follower_count stay accurate despite the double LEFT JOIN.
@end

@case
id: FE-0527
surface: /photographers
catégorie: contrat-API
tag: back
précondition: decode::<CountCursor>/DateCursor returns Option (q.cursor.as_deref().and_then(decode)); a failed decode yields None, not an error.
action: GET /api/photographers?cursor=@@@invalid
attendu: decode returns None so the query runs as the first page (cursor predicate `$1 is null` true) with 200 — unlike explore/category/tag which 400 on cursor_invalid; this is a deliberate contract difference.
@end

@case
id: FE-0528
surface: /photographers
catégorie: frontend
tag: front
précondition: +page.svelte renders cover via cover_photo_id (PhotographerListItem.cover_photo_id is Option<Uuid>.map(to_string)); From<Row> sets member_since_year = created_at year parsed, unwrap_or(2026).
action: render a photographer whose cover_photo_id is null
attendu: the From impl emits cover_photo_id: None; the tile must render without a cover image (placeholder), and member_since_year falls back to 2026 only if year parse fails.
@end

@case
id: FE-0529
surface: /photographers
catégorie: sécurité
tag: back
précondition: handle is selected as u.handle::text and display_name as u.display_name; sort is matched against literal arms, cursor is base64-decoded JSON — no value is concatenated into SQL.
action: GET /api/photographers?sort=active'; drop table users;-- 
attendu: sort hits the `_ =>` active branch (string compared, not interpolated); no SQL injection; handle/display_name are escaped by Svelte on render so a malicious display_name cannot XSS the index.
@end

@case
id: FE-0530
surface: /c/[cat]
catégorie: entrées-limites
tag: back
précondition: category.rs limit = q.limit.unwrap_or(24).clamp(1,60); sort defaults to "newest", unknown sort hits `_ =>` newest branch.
action: GET /api/categories/dso?limit=-1&sort=weird
attendu: limit clamps to 1; sort routes to the newest query (`order by p.published_at desc, p.id desc`); response shape is CategoryPage{category, photo_count, page}.
@end

@case
id: FE-0531
surface: /c/[cat]
catégorie: entrées-limites
tag: back
précondition: +page.server.ts forwards only sort+limit; it explicitly drops `since` because /api/categories/:cat does not implement it.
action: GET /c/dso?since=24h
attendu: the since param is ignored at the loader (not forwarded), so the category feed is unfiltered by time; this is intentional (comment: forwarding it would silently no-op).
@end

@case
id: FE-0532
surface: /c/[cat]
catégorie: états
tag: back
précondition: category.rs computes photo_count via count(*) where category=$1 and published_at not null and status='ready'; a valid category with no photos returns photo_count=0 and empty page.photos.
action: GET /c/solar when no solar photos are published
attendu: 200 CategoryPage with photo_count=0 and page.photos=[] (valid-but-empty), distinct from an unknown category which 404s.
@end

@case
id: FE-0533
surface: /c/[cat]
catégorie: concurrence
tag: back
précondition: photo_count is fetched in a separate query before the feed rows query (two statements, not one snapshot/transaction).
action: a photo in the category is published between the count query and the feed query
attendu: CategoryPage.photo_count can be one less than the rows actually returned (or vice versa); the count and the page are read at slightly different instants with no enclosing transaction.
@end

@na
surface: /c/[cat]
catégorie: auth-tenant
raison: category.rs get() takes only State, Path(cat), Query<Q> — no user extractor; both queries hard-filter `published_at is not null and status='ready'`, so output is viewer-independent and drafts/other-tenant photos cannot appear. No auth/tenant dimension to exercise.
@end

@case
id: FE-0534
surface: /c/[cat]
catégorie: intégrité-données
tag: back
précondition: category.rs `let cat = cat.replace('-', "_")` then `if !CATEGORIES.contains(&cat.as_str()) return not_found`; CATEGORIES holds the underscore DB-enum forms.
action: GET /api/categories/wide-field
attendu: hyphen→underscore normalization maps it to "wide_field" which is in CATEGORIES; the SQL filters p.category = 'wide_field' (the stored enum value), so the slug→category mapping resolves correctly rather than 404ing.
@end

@case
id: FE-0535
surface: /c/[cat]
catégorie: contrat-API
tag: back
précondition: category.rs returns AppError::not_found("category") for unknown cat; fetchCategoryPage throws Error('not_found') on 404; +page.server.ts catches message==='not_found' → error(404).
action: GET /c/galaxies (not in CATEGORIES)
attendu: backend 404 not_found("category") → client throws 'not_found' → SvelteKit error(404, 'Category not found'); a missing slug is a hard 404, not an empty 200.
@end

@case
id: FE-0536
surface: /c/[cat]
catégorie: frontend
tag: front
précondition: +page.server.ts mirrors the backend by replacing /-/g with '_' before checking VALID_CATEGORIES, to avoid 404ing /c/wide-field that the sitemap/home pills link.
action: navigate to a home-page category pill linking /c/wide-field
attendu: the loader normalizes to wide_field, passes the VALID_CATEGORIES check, and calls fetchCategoryPage(fetch,'wide_field',...) — the hyphenated URL renders instead of erroring.
@end

@case
id: FE-0537
surface: /c/[cat]
catégorie: sécurité
tag: back
précondition: cat is validated against the fixed CATEGORIES allowlist before any SQL; only an exact enum match reaches `p.category = $1` (bound param).
action: GET /api/categories/dso%27%20OR%201=1--
attendu: the decoded path value is not in CATEGORIES → 404 not_found before SQL; even a category that passed would be a bound parameter, so path-based SQL injection is impossible.
@end

@case
id: FE-0538
surface: /tag/[slug]
catégorie: entrées-limites
tag: back
précondition: tag.rs limit = q.limit.unwrap_or(24).clamp(1,60); supports an extra `category` filter `($5::text is null or p.category = $5)`.
action: GET /api/tags/nebula?limit=1000&category=planetary
attendu: limit clamps to 60; rows additionally filtered to p.category='planetary'; unknown category value simply matches nothing (no error).
@end

@case
id: FE-0539
surface: /tag/[slug]
catégorie: états
tag: back
précondition: tag.rs resolves the tag row first (fetch_optional); photo_count is a correlated subquery over photo_tags joined to published/ready photos.
action: GET /api/tags/<existing slug with zero published photos>
attendu: 200 TagPage with tag.photo_count=0 and page.photos=[] — the tag exists so it is not a 404, just an empty feed.
@end

@case
id: FE-0540
surface: /tag/[slug]
catégorie: concurrence
tag: back
précondition: tag.rs reads TagMeta.photo_count (subquery) and the feed rows in two separate statements; newest feed uses `(p.published_at,p.id) < ($2,$3)` keyset.
action: a photo gains/loses this tag (photo_tags insert/delete) between the meta query and the feed query, or while paginating with a cursor
attendu: TagMeta.photo_count may diverge from the rendered row count; keyset pagination on (published_at,id) stays stable for rows already below the cursor regardless of new tag attachments above it.
@end

@na
surface: /tag/[slug]
catégorie: auth-tenant
raison: tag.rs get() has no user extractor (State, Path(slug), Query<Q>); the feed join filters `p.published_at is not null and p.status='ready'` so untagged-draft and other-users' unpublished photos are never returned. Output is viewer-independent — no tenant dimension.
@end

@case
id: FE-0541
surface: /tag/[slug]
catégorie: intégrité-données
tag: back
précondition: the slug uniquely identifies a tag row (`where t.slug = $1` via fetch_optional); the feed joins photo_tags pt on pt.tag_id = t.id (the resolved id), not on the slug string.
action: GET /api/tags/orion-nebula
attendu: slug→tag.id mapping is resolved once and the photo join uses t.id; renaming display `name` later does not change which photos the slug returns, so slug→content mapping stays consistent.
@end

@case
id: FE-0542
surface: /tag/[slug]
catégorie: contrat-API
tag: back
précondition: tag.rs returns AppError::not_found("tag") when fetch_optional yields None; fetchTagPage throws Error('not_found') on 404; +page.server.ts maps message==='not_found' → error(404,'Tag not found').
action: GET /tag/does-not-exist
attendu: backend 404 not_found("tag") → client 'not_found' → SvelteKit 404 page; a nonexistent slug is a hard 404, never an empty 200 TagPage.
@end

@case
id: FE-0543
surface: /tag/[slug]
catégorie: frontend
tag: front
précondition: +page.server.ts reads url.searchParams.get('category') and forwards it as an optional FeedOpts.category only when non-null; sort defaults to 'newest'.
action: load /tag/widefield?category=nightscape&sort=most-appreciated
attendu: loader builds fetchTagPage(fetch, slug, {sort:'most-appreciated', category:'nightscape', limit:24}); both params forwarded; with exactOptionalPropertyTypes the category key is omitted (not undefined) when absent.
@end

@case
id: FE-0544
surface: /tag/[slug]
catégorie: sécurité
tag: back
précondition: slug is bound to `where t.slug = $1` (parameterized); cursor is base64 JSON decoded with cursor_invalid on failure; category is a bound `$5::text` filter.
action: GET /api/tags/'; drop table tags;--  and GET /api/tags/x?cursor=AAAA
attendu: the malicious slug is a literal bound param → no row matches → 404 not_found("tag"), no injection; a malformed cursor returns 400 cursor_invalid before SQL.
@end

@case
id: FE-0545
surface: /
catégorie: entrées-limites
tag: back
précondition: home +page.server.ts calls /api/photos?following=true (authed) then /api/photos?limit=24; there are no user-controlled query params on `/` itself.
action: load / with no query string, or with junk like /?foo=bar
attendu: junk params are ignored; the loader fetches /api/photos?limit=24 (public) or the following feed; the page renders the real gallery or, on backend-down, the PHOTOS/NGC7000 placeholder demo content.
@end

@case
id: FE-0546
surface: /
catégorie: états
tag: front
précondition: home +page.server.ts: if realPhotos.length>0 returns isReal:true gallery, else returns isReal:false placeholder (PHOTOS.slice(0,12), heroSrc:undefined).
action: brand-new site with zero published photos
attendu: realPhotos stays [] so the loader returns isReal:false with NGC7000 hero and demo PHOTOS — the landing is never empty even with no content.
@end

@case
id: FE-0547
surface: /
catégorie: concurrence
tag: back
précondition: statsPromise (/api/site/stats) runs in parallel with the photos fetch and is awaited via `await statsPromise`; the feed list is a separate read.
action: photos publish between the stats read and the /api/photos read
attendu: stats.frames (eyebrow) and the rendered hero/grid count can drift; stats is a separate site-wide aggregate, not derived from the returned photo list.
@end

@case
id: FE-0548
surface: /
catégorie: auth-tenant
tag: back
précondition: home loader passes the inbound Cookie header to /api/photos?following=true only when locals.user is set; falls back to public /api/photos?limit=24 when the personalised feed is empty.
action: authed user who follows nobody loads /
attendu: following=true returns an empty photos array, realPhotos stays [], so the loader falls through to the public feed (limit=24) — the personalised path degrades to public, and only published photos are ever returned (no draft leak).
@end

@na
surface: /
catégorie: intégrité-données
raison: the home loader does no counting/joins of its own — it forwards /api/photos rows straight through and reads stats.frames as an opaque scalar; there is no count-vs-rows or slug-mapping invariant owned by `/` to violate (those live in /api/photos and /api/site/stats). Covered under those endpoints, not here.
@end

@case
id: FE-0549
surface: /
catégorie: contrat-API
tag: back
précondition: home loader treats /api/photos and /api/site/stats as best-effort: each fetch is in try/catch (or .catch(()=>null)) and only consumed `if (res.ok)`.
action: /api/photos returns 500 or times out during SSR
attendu: the catch swallows it, realPhotos stays [], and the loader returns the isReal:false placeholder — `/` never throws a 500 error page even when the backend feed endpoint fails (unlike /explore which does throw).
@end

@case
id: FE-0550
surface: /
catégorie: frontend
tag: front
précondition: home builds thumbSrc via cdn(p.id,{w:400}) and heroSrc via cdn(hero.id,{w:1200}); feeds are SSR so tiles are present in the server-rendered HTML.
action: SSR-fetch / and inspect the HTML
attendu: the gallery <img> tags carry CDN URLs from cdn(id,{w}) and ratio width/height attrs rendered server-side; tiles are in the initial HTML (not client-only), so SSR shows non-empty content.
@end

@case
id: FE-0551
surface: /
catégorie: sécurité
tag: front
précondition: home renders target/original_name/photographer as escaped Svelte text; heroSrc/thumbSrc are cdn()-built URLs keyed on photo id (a UUID), not on user free-text.
action: a photo with a malicious original_name (e.g. "<img onerror>") surfaces on the home feed
attendu: original_name is interpolated as escaped text (no raw HTML) and the image src derives from the UUID via cdn(), so neither the caption nor the src can inject script; no private/draft photo reaches `/` because /api/photos returns only published rows.
@end

<!-- ===== batch F ===== -->

@case
id: FE-0600
surface: /t
catégorie: entrées-limites
tag: back
précondition: catalog seeded with ~12k OpenNGC targets
action: GET /api/targets?q=<2000-char string>&sort=popular through fetchTargetList in /t/+page.server.ts load
attendu: handler treats q as Option<String> in discovery::target_index Q and runs an ILIKE; no 500 — load returns initial with limit 24, has_photos defaults to true
@end

@case
id: FE-0601
surface: /t
catégorie: états
tag: back
précondition: dev DB where seed-targets never ran (targets.right_ascension is NULL)
action: GET /t?sort=optimal (planning=true forces fullCatalog, drops has_photos=true)
attendu: opposition/cone-search render empty because right_ascension/opposition_doy are NULL in dev; +page.server.ts still returns initial, page shows full-catalog stubs
@end

@case
id: FE-0602
surface: /t
catégorie: concurrence
tag: back
précondition: two photos of the same target published seconds apart
action: two parallel publish flows insert into photo_targets for the same target_id while /t computes photo_count
attendu: /t list sort is recomputed per request via fetchTargetList; photo_count subquery counts photos with published_at is not null and status='ready' — no lost-update, count reflects committed rows at query time
@end

@na
surface: /t
catégorie: auth-tenant
raison: /t/+page.server.ts load passes no cookie and fetchTargetList hits the public /api/targets index which has no auth guard — there is no per-tenant or owner dimension on the catalog browse list to abuse.
@end

@case
id: FE-0603
surface: /t
catégorie: intégrité-données
tag: back
précondition: a target with 3 photos, 1 still status='processing'
action: GET /api/targets index entry for that slug computes photo_count
attendu: target_index photo_count filter requires p.status='ready' AND p.published_at is not null, so the processing photo is excluded — list count matches the ready/published rows only
@end

@case
id: FE-0604
surface: /t
catégorie: contrat-API
tag: back
précondition: sort param outside the index's known set
action: GET /t?sort=garbage forwarded to fetchTargetList
attendu: backend defaults unknown sort to 'popular' equivalent (no 422); /t load wraps any thrown fetch in error(500,'Failed to load targets'), so a real backend failure is a 500 page not a blank
@end

@case
id: FE-0605
surface: /t
catégorie: frontend
tag: front
précondition: catalog query returns zero rows (e.g. ?q=zzzznotarget)
action: SSR render of /t +page.svelte with initial.targets empty
attendu: empty-state branch renders (no tiles); the size-bucket/Optimal-now planning toggles still show because planning forces fullCatalog in +page.server.ts
@end

@case
id: FE-0606
surface: /t
catégorie: sécurité
tag: back
précondition: attacker supplies object_type/constellation query params
action: GET /t?object_type=' OR 1=1--&constellation=<script>
attendu: params bound as sqlx parameters in discovery::target_index (no string interpolation); constellation rendered text-interpolated in +page.svelte, no XSS, no injection
@end

@case
id: FE-0607
surface: /t/[slug]
catégorie: entrées-limites
tag: back
précondition: no target row with that slug
action: GET /api/targets/UPPERCASE-OR-bogus-slug via fetchTargetPage in /t/[slug]/+page.server.ts
attendu: discovery::target::get returns Err(AppError::not_found("target")); client maps message 'not_found' to throw error(404,'Target not found')
@end

@case
id: FE-0608
surface: /t/[slug]
catégorie: états
tag: back
précondition: a real catalog target (slug exists) but zero published photos
action: GET /api/targets/<slug>
attendu: discovery::target::get returns 200 with TargetMeta (photo_count=0, contributor_count=0) and page.photos=[]; 404 is reserved for a missing targets row, not an empty gallery
@end

@case
id: FE-0609
surface: /t/[slug]
catégorie: concurrence
tag: back
précondition: paginating with a cursor while a new photo is published into the same target
action: GET /api/targets/<slug>?sort=newest with a stale cursor after a concurrent publish
attendu: keyset predicate (p.published_at, p.id) < ($2,$3) in target::get is monotonic on published_at desc, id desc — the new row sorts above the cursor and is simply not re-served; no duplicate, no skip of older rows
@end

@na
surface: /t/[slug]
catégorie: auth-tenant
raison: target::get takes no OptionalUser/CurrentUser and only joins published+ready photos; the page is fully public with no owner/tenant branch, so there is no auth boundary to cross.
@end

@case
id: FE-0610
surface: /t/[slug]
catégorie: intégrité-données
tag: back
précondition: a draft photo plate-solved onto the target (photo_targets row exists, photos.published_at is NULL)
action: GET /api/targets/<slug>
attendu: target::get gallery query requires p.published_at is not null and p.status='ready', so the draft is excluded from page.photos and from photo_count/contributor_count — no draft leakage via the target gallery
@end

@case
id: FE-0611
surface: /t/[slug]
catégorie: contrat-API
tag: back
précondition: category filter param present
action: GET /api/targets/<slug>?category=widefield&sort=most-appreciated
attendu: target::get binds category as ($5::text is null or p.category=$5); response is TargetPage{target:TargetMeta, page:DiscoveryPage{photos,next_cursor}} — shape stable; limit clamped to MAX_LIMIT=60
@end

@case
id: FE-0612
surface: /t/[slug]
catégorie: frontend
tag: front
précondition: backend forwarded a since param the page does not implement
action: open /t/[slug]?since=7d
attendu: +page.server.ts deliberately does not forward since (comment: /api/targets/:slug does not implement it); the param silently no-ops, gallery shows newest/most-appreciated as selected, no error
@end

@case
id: FE-0613
surface: /t/[slug]
catégorie: sécurité
tag: front
précondition: target canonical_name or aliases contain HTML metacharacters in the catalog row
action: SSR render of /t/[slug] +page.svelte with t.canonical_name = "M31 <img onerror>"
attendu: canonical_name/aliases bound as sqlx params and rendered via Svelte text interpolation (not @html), so the markup is escaped — no stored XSS through the target name
@end

@case
id: FE-0614
surface: /equip/[kind]
catégorie: entrées-limites
tag: front
précondition: kind segment not in the allowlist
action: navigate /equip/binoculars
attendu: +page.server.ts checks VALID_KINDS (telescope|camera|mount|filter|focal_modifier|guiding) and throws error(404,'Unknown equipment kind') before any backend call
@end

@case
id: FE-0615
surface: /equip/[kind]
catégorie: états
tag: front
précondition: valid kind, page index far past the last result
action: GET /equip/telescope?page=99999
attendu: safePage = Math.floor of a finite >=0 Number; backend /api/equipment/catalog returns an empty items page (200); +page.svelte renders the empty grid, not a 500
@end

@na
surface: /equip/[kind]
catégorie: concurrence
raison: the catalog browse list is a read-only aggregate over equipment_items.usage_count; no mutation is issued from /equip/[kind], so there is no write-write or read-write race specific to this surface (covered under the detail/edit surfaces).
@end

@na
surface: /equip/[kind]
catégorie: auth-tenant
raison: /equip/[kind]/+page.server.ts performs no auth check and the /api/equipment/catalog endpoint is public global catalog data with no owner dimension — nothing tenant-scoped to bypass.
@end

@case
id: FE-0616
surface: /equip/[kind]
catégorie: intégrité-données
tag: back
précondition: an equipment_item with usage_count drifted above its real photo references
action: GET /api/equipment/catalog?kind=telescope&sort=most_used
attendu: catalog ranks by usage_count column (denormalized); a tile's "used by N" reflects usage_count, which must be kept in sync by the upsert/apply-setup writers — drift surfaces as a mis-ordered most_used list
@end

@case
id: FE-0617
surface: /equip/[kind]
catégorie: contrat-API
tag: front
précondition: backend catalog endpoint returns a non-200 (e.g. 422 on bad min_aperture)
action: GET /equip/camera?min_aperture=abc
attendu: +page.server.ts forwards min_aperture raw; if r.ok is false it throws error(r.status,...) preserving the backend status; only a thrown non-status error falls through to error(500,'Failed to load catalog')
@end

@case
id: FE-0618
surface: /equip/[kind]
catégorie: frontend
tag: front
précondition: sort param not in ALLOWED_SORTS
action: GET /equip/mount?sort=cheapest
attendu: +page.server.ts coerces sort to 'most_used' via ALLOWED_SORTS.has check before building params2; the select renders most_used as active, no invalid sort reaches the API
@end

@case
id: FE-0619
surface: /equip/[kind]
catégorie: sécurité
tag: front
précondition: attacker injects q with markup
action: GET /equip/telescope?q=<svg onload=alert(1)>
attendu: q is trimmed and passed as a URLSearchParams value (encoded) to /api/equipment/catalog; rendered back via text interpolation in +page.svelte — no reflected XSS
@end

@case
id: FE-0620
surface: /equip/[kind]/[slug]
catégorie: entrées-limites
tag: back
précondition: slug that matches no catalog canonical_name for the kind
action: GET /api/equipment/telescope/not-a-real-scope?limit=24
attendu: discovery::equipment::get returns 404; +page.server.ts maps discoveryR.status===404 to throw error(404,'Equipment item not found')
@end

@case
id: FE-0621
surface: /equip/[kind]/[slug]
catégorie: états
tag: back
précondition: a multi-filter photo with photos.filters="L,R,G,B" and the single-filter slug "l"
action: GET /equip/filter/l detail page
attendu: discovery handler matches the legacy photos.filters text cache by exact lower(filters)=slug, so the L,R,G,B photo does NOT appear under slug "l" — documented known limitation in +page.server.ts
@end

@case
id: FE-0622
surface: /equip/[kind]/[slug]
catégorie: concurrence
tag: back
précondition: an admin renames this item (PATCH display_name) while a visitor loads the detail page
action: concurrent GET /api/equipment/items/:id (step 2 of load) during items_update PATCH that holds `for update` lock and calls filters_cache::rebuild_for_item
attendu: the row lock serializes; the GET either sees the pre- or post-commit canonical_name atomically, never a half-applied rename (specs delete+insert are in the same tx)
@end

@case
id: FE-0623
surface: /equip/[kind]/[slug]
catégorie: auth-tenant
tag: front
précondition: an anonymous (logged-out) visitor
action: load /equip/telescope/<slug>
attendu: +page.server.ts sets canSeeEditAffordance = !!user (from parent layout.user); anonymous user gets false so the "Edit specs" link is hidden — but the real gate is the admin check on the /edit route, not this affordance
@end

@case
id: FE-0624
surface: /equip/[kind]/[slug]
catégorie: intégrité-données
tag: back
précondition: an item whose kind has no matching specs sub-table row
action: GET /api/equipment/items/:id for a telescope with no telescope_specs row
attendu: items_get load_telescope fetch_optional returns None → specs:null in EquipmentItemDetail; the fiche renders without spec rows rather than erroring
@end

@case
id: FE-0625
surface: /equip/[kind]/[slug]
catégorie: contrat-API
tag: front
précondition: discovery resolves but the items/:id hydrate call fails
action: GET /equip/camera/<slug> where /api/equipment/items/:id returns 500
attendu: +page.server.ts throws error(500,'Failed to load catalog item detail'); a discovery 404 vs a hydrate 500 are distinguished — 404 only when discoveryR.status===404
@end

@case
id: FE-0626
surface: /equip/[kind]/[slug]
catégorie: frontend
tag: front
précondition: detail page resolved with photo tiles
action: render +page.svelte photo tiles linking each photo
attendu: each PhotoTile carries author_handle + short_id from discovery.page.photos, so links go to /u/<author_handle>/p/<short_id> with no extra round-trip (the reason load uses the discovery handler)
@end

@case
id: FE-0627
surface: /equip/[kind]/[slug]
catégorie: sécurité
tag: back
précondition: slug contains a path-traversal / encoded segment
action: GET /equip/telescope/..%2F..%2Fadmin
attendu: +page.server.ts wraps with encodeURIComponent(params.slug) before building the /api/equipment/:kind/:slug URL; backend treats it as a literal slug, no row matches → 404, no traversal
@end

@case
id: FE-0628
surface: /equip/[kind]/[slug]/edit
catégorie: entrées-limites
tag: back
précondition: admin user, display_name submitted as whitespace only
action: PATCH /api/equipment/items/:id with EquipmentItemPatch.display_name = "   "
attendu: items_update trims then returns AppError::Validation("display_name cannot be empty") and the tx rolls back; no rename, no rebuild_for_item
@end

@case
id: FE-0629
surface: /equip/[kind]/[slug]/edit
catégorie: états
tag: back
précondition: admin, item id no longer exists (deleted between list and edit)
action: PATCH /api/equipment/items/:id for a missing id
attendu: items_update's `select kind ... for update` fetch_optional is None → AppError::NotFound("equipment item not found"); the edit form save shows a 404, not a 500
@end

@case
id: FE-0630
surface: /equip/[kind]/[slug]/edit
catégorie: concurrence
tag: back
précondition: two admins editing the same item; admin B renames it to a name that becomes a duplicate after admin A committed
action: PATCH display_name colliding on the (kind, canonical_name) unique index
attendu: items_update catches sqlx::Error::Database with constraint().is_some() → AppError::Conflict("another item of this kind already uses that name") (409), not a 500
@end

@case
id: FE-0631
surface: /equip/[kind]/[slug]/edit
catégorie: auth-tenant
tag: front
précondition: a signed-in but non-admin user
action: navigate /equip/telescope/<slug>/edit
attendu: edit/+page.server.ts checks locals.user.isAdmin and throws error(403,'Catalog editing requires an admin account'); a logged-out user instead gets redirect(303,'/signin?next=/equip/.../edit')
@end

@case
id: FE-0632
surface: /equip/[kind]/[slug]/edit
catégorie: intégrité-données
tag: back
précondition: admin changes specs whose payload kind disagrees with the item's stored kind
action: PATCH /api/equipment/items/:id (telescope) with EquipmentItemPatch.specs of camera shape
attendu: items_update calls specs::ensure_matches_kind(&row.kind, payload) before writing; mismatch returns 422 and the whole tx (including any rename) rolls back clean — no orphaned specs row
@end

@case
id: FE-0633
surface: /equip/[kind]/[slug]/edit
catégorie: contrat-API
tag: back
précondition: admin sends an empty PATCH body (neither display_name nor specs)
action: PATCH /api/equipment/items/:id with {}
attendu: per items_update doc, omitting both is a no-op that still returns the current item via items_get::handler (200), not a 400
@end

@case
id: FE-0634
surface: /equip/[kind]/[slug]/edit
catégorie: frontend
tag: front
précondition: admin renamed the item; photos.filters cache must reflect it
action: PATCH display_name on a filter item then reload an affected photo's detail
attendu: items_update calls crate::photos::filters_cache::rebuild_for_item(&mut tx, id) inside the same tx so photos.filters denormalized cache is rebuilt; stale filter chip names do not persist
@end

@case
id: FE-0635
surface: /equip/[kind]/[slug]/edit
catégorie: sécurité
tag: back
précondition: non-admin crafts the PATCH directly, bypassing the SvelteKit /edit guard
action: PATCH /api/equipment/items/:id with a valid session but is_admin=false
attendu: handler signature takes _admin: AdminUser extractor — the request is rejected at the guard (403) before reaching the tx; the frontend 403 is defense-in-depth, the backend AdminUser guard is the real boundary
@end

@case
id: FE-0636
surface: /u/[handle]
catégorie: entrées-limites
tag: back
précondition: handle supplied in mixed case
action: GET /api/users/<MixedCaseHandle> via fetchPublicProfile
attendu: public_profile::get queries where handle = $1 with handle.to_lowercase(), so case is normalized server-side and the profile resolves regardless of input case
@end

@case
id: FE-0637
surface: /u/[handle]
catégorie: états
tag: back
précondition: a user who renamed their handle (old handle recorded in redirect history)
action: GET /u/<old-handle>
attendu: fetchPublicProfile throws 'not_found'; +page.server.ts then GET /api/handles/redirect/<handle> and on ok throws redirect(301, `/u/<target>`); otherwise error(404,'No photographer here.')
@end

@case
id: FE-0638
surface: /u/[handle]
catégorie: concurrence
tag: back
précondition: profile loads while owner publishes a photo
action: parallel fetchPublicProfile (stats query) and a concurrent publish updating photos.appreciations_count
attendu: stats_row aggregates frames/integration/appreciations/targets in one query filtered on published_at is not null and status='ready'; the snapshot is consistent at query time — a just-published photo may or may not be counted but never double-counted
@end

@case
id: FE-0639
surface: /u/[handle]
catégorie: auth-tenant
tag: front
précondition: viewer is the profile owner vs a visitor
action: load /u/<own-handle> while signed in
attendu: +page.server.ts sets isSelf = locals.user?.id === profile.id and viewMode = isSelf ? 'owner' : 'visitor'; owner-only affordances key off viewMode, public profile data is identical either way
@end

@case
id: FE-0640
surface: /u/[handle]
catégorie: intégrité-données
tag: back
précondition: cover_photo_id points to a photo, plus featured photos exist
action: GET /api/users/<handle>
attendu: public_profile::get loads featured ordered by featured_position and cover via cover_photo_id separately; if cover_photo_id is null, cover=None — the fiche never fabricates a cover, and featured_position defaults to 0 when null
@end

@case
id: FE-0641
surface: /u/[handle]
catégorie: contrat-API
tag: back
précondition: handle that is actually a pasted user UUID
action: GET /u/<uuid>
attendu: +page.server.ts UUID_RE matches, GET /api/users/<uuid>; on ok throws redirect(301, `/u/<target-handle>`); on miss falls through to the not_found path — distinguishes a UUID paste (301) from an unknown handle (404)
@end

@case
id: FE-0642
surface: /u/[handle]
catégorie: frontend
tag: front
précondition: profile with social_links and tagline
action: SSR render /u/[handle] +page.svelte head
attendu: title derived as `${display_name} — Astrophoto`, og/meta description from tagline; JSON-LD emitted via ldJsonScriptTag(jsonLd) with knowsAbout only when bio_html present
@end

@case
id: FE-0643
surface: /u/[handle]
catégorie: sécurité
tag: front
précondition: a user whose bio contains a stored <script>
action: render /u/[handle] HeroAbout with profile.bio_html
attendu: bio_html was passed through users::bio::sanitize (ammonia cleaner strips script/onclick/javascript: per its tests) at write time; HeroAbout's {@html bio} is safe because the value is server-sanitised — display_name/tagline are text-interpolated
@end

@case
id: FE-0644
surface: /u/[handle]/p/[shortid]
catégorie: entrées-limites
tag: back
précondition: a short_id that does not exist under the handle
action: GET /api/photos/by-permalink/<handle>/<bogus-shortid>
attendu: permalink::lookup query returns no row (also requires p.published_at is not null), .ok_or_else → AppError::NotFound("photo"); load then tries handle redirect, else error(404,'Photo not found')
@end

@case
id: FE-0645
surface: /u/[handle]/p/[shortid]
catégorie: états
tag: back
précondition: a draft photo (published_at IS NULL) whose short_id is known
action: GET /api/photos/by-permalink/<owner-handle>/<draft-shortid>
attendu: permalink::lookup requires p.published_at is not null, so a draft never resolves via the public permalink — returns 404 even to the owner; the draft is reachable only via the authenticated /api/photos/:id path
@end

@case
id: FE-0646
surface: /u/[handle]/p/[shortid]
catégorie: concurrence
tag: back
précondition: a user double-clicks the heart (two appreciate POSTs in flight)
action: two concurrent POST /api/photos/:id/appreciate from the same user
attendu: insert ... on conflict (user_id, photo_id) do nothing makes the second a 0-row insert; the photos.appreciations_count increment is gated on rows_affected>0, so the count rises by exactly 1 — idempotent toggle
@end

@case
id: FE-0647
surface: /u/[handle]/p/[shortid]
catégorie: auth-tenant
tag: back
précondition: attacker enumerates short_ids of another user's drafts (IDOR attempt)
action: GET /api/photos/by-permalink/<victim>/<guessed-shortid> then GET /api/photos/:id as a stranger
attendu: permalink requires published_at not null; /api/photos/:id calls queries::is_visible_to which returns false for a draft viewed by a non-owner → AppError::not_found("photo"); pipeline_error is also nulled for non-owners — no draft leakage
@end

@case
id: FE-0648
surface: /u/[handle]/p/[shortid]
catégorie: intégrité-données
tag: back
précondition: a photo whose denormalized appreciations_count drifted below the appreciations row count
action: GET /api/photos/:id (detail)
attendu: photos::get::handler recomputes appreciation_count and comment_count with live select count(*) from appreciations/comments (not the denormalized column), so the detail page shows the true row count; unappreciate uses greatest(count-1,0) to prevent negative drift
@end

@case
id: FE-0649
surface: /u/[handle]/p/[shortid]
catégorie: contrat-API
tag: back
précondition: a comment posted with an empty or 2001-char body
action: POST /api/photos/:id/comments with body "" then body of length 2001
attendu: CreateBody #[validate(length(min=1, max=2000))]; on failure create returns AppError::Validation(...) (422); success returns 201 CREATED with the Comment JSON {id,photo_id,author_id,author_display_name,body,created_at}
@end

@case
id: FE-0650
surface: /u/[handle]/p/[shortid]
catégorie: frontend
tag: front
précondition: a published photo with ra_deg set and celestial objects + platesolveStatus
action: SSR render of permalink → PhotoDetailFull renders CelestialOverlay when celestial.length>0 && solveForOverlay && overlayEnabled
attendu: the overlay needs WCS (ra/dec/pixel_scale/rotation) from /api/photos/:id/platesolve-status, which is fetched in load only when photo.ra_deg != null; absent a solve, the overlay simply does not render
@end

@case
id: FE-0651
surface: /u/[handle]/p/[shortid]
catégorie: sécurité
tag: front
précondition: a comment body containing <script>alert(1)</script>
action: post the comment, reload the thread
attendu: CommentThread renders {c.body} via Svelte text interpolation (not @html), so the markup is escaped — stored XSS via comment body is neutralised at render; caption is likewise rendered as {p.caption} text
@end

@case
id: FE-0652
surface: /u/[handle]/p/[shortid]
catégorie: états
tag: back
précondition: a comment whose author deleted their account (author_id NULL)
action: GET /api/photos/:id/comments
attendu: comments::list left-joins users and coalesces display_name to '[deleted]'; the comment still lists with author_id=null; delete of such a comment is allowed only to the photo owner (author_id != Some(user.id) && photo_owner_id != user.id → AppError::Forbidden)
@end

@case
id: FE-0653
surface: /u/[handle]/p/[shortid]
catégorie: concurrence
tag: back
précondition: a flooded photo with thousands of comments
action: GET /api/photos/:id/comments
attendu: comments::list caps at LIST_LIMIT=200, selecting the newest 200 (order by created_at desc, id desc) re-sorted ascending; older comments are unreachable until a cursor ships — prevents unbounded buffering on a public endpoint
@end

@case
id: FE-0654
surface: /u/[handle]/p/[shortid]
catégorie: auth-tenant
tag: back
précondition: a logged-out visitor clicks the heart
action: POST /api/photos/:id/appreciate with no session
attendu: appreciate handler takes CurrentUser(user) extractor → 401 Unauthorized when unauthenticated; reading the count (GET /appreciations/count) uses OptionalUser and stays public for published photos
@end

<!-- ===== batch G1 ===== -->

@case
id: FE-0700
surface: /about
catégorie: entrées-limites
tag: front
précondition: navigation directe vers /about avec une query string arbitraire, ex. /about?foo=<script> ou /about#frag
action: charger la route ; le +page.svelte de /about n'a ni load, ni $page.url lu, ni paramètre exploité
attendu: la query/hash est ignorée — le <main class="static-page"> rend le H1 "A quiet archive of the night sky." sans réfléter foo ; aucune entrée utilisateur n'atteint le DOM
@end

@na
surface: /about
catégorie: états
raison: /about/+page.svelte n'a aucun $state, aucun await, aucun bind — le contenu est du markup statique (H1 + 3 <p> + .t-meta) ; il n'existe pas d'état loading/empty/error/success à reproduire sur cette page
@end

@na
surface: /about
catégorie: concurrence
raison: aucun +page.server.ts, aucune form action, aucun fetch dans /about/+page.svelte ; il n'y a pas deux écritures possibles, donc pas de course (double-submit, write-write, invalidateAll) à reproduire
@end

@case
id: FE-0701
surface: /about
catégorie: auth-tenant
tag: front
précondition: deux sessions — anonyme, puis connectée (page.data.user non null via hooks.server.ts)
action: charger /about dans chaque session ; <AppHeader active="About" /> lit `user = $derived(page.data.user)`
attendu: la zone droite du header bascule — anonyme: liens "Sign in" + "Create account" ; connecté: bouton "Upload" + <AvatarMenu>. Le corps de /about (texte) est identique pour tous ; aucune donnée tenant-spécifique n'est rendue dans <main>
@end

@na
surface: /about
catégorie: intégrité-données
raison: /about ne lit ni n'écrit aucune donnée (pas de query!, pas de finalize, pas de cache photos.filters) ; le texte est codé en dur dans +page.svelte — il n'y a pas d'invariant de persistance ou de cache dénormalisé à violer
@end

@na
surface: /about
catégorie: contrat-API
raison: /about/+page.svelte n'émet aucun appel réseau (pas de fetch, pas de +page.server.ts, pas d'action) ; il n'y a aucun contrat requête/réponse à valider — seul AppHeader déclenche api.me, mais en amont dans hooks.server.ts, hors de cette route
@end

@case
id: FE-0702
surface: /about
catégorie: frontend
tag: front
précondition: viewport 640px ou moins (mobile)
action: rendre /about ; la media query @media (max-width: 640px) dans le <style> réduit .static-page h1 à font-size 32px et le padding à 48px 24px 64px
attendu: le H1 "A quiet archive of the night sky." passe de 48px à 32px sans débordement horizontal ; <MobileNav> du header remplace .primary-nav (masquée <768px) et reste atteignable
@end

@case
id: FE-0703
surface: /about
catégorie: sécurité
tag: front
précondition: réponse HTTP de /about inspectée (headers) en prod
action: vérifier que hooks.server.ts applique les securityHeaders à la réponse SSR de /about
attendu: présence de X-Frame-Options: SAMEORIGIN, X-Content-Type-Options: nosniff, Content-Security-Policy-Report-Only (frame-ancestors 'none') ; le texte de /about ne contient aucun {@html} ni style= avec donnée externe, donc aucun sink XSS sur cette page
@end

@case
id: FE-0704
surface: /contact
catégorie: entrées-limites
tag: front
précondition: page /contact rendue
action: cliquer le lien <a href="mailto:hello@astrophoto.example"> ; aucune saisie utilisateur n'existe sur la page (pas de <input>, pas de form)
attendu: le navigateur ouvre le client mail vers hello@astrophoto.example ; il n'y a aucun champ à valider/borner côté app — la limite est déléguée au client mail
@end

@na
surface: /contact
catégorie: états
raison: /contact/+page.svelte ne contient ni $state, ni form, ni soumission ; le seul élément interactif est un lien mailto + un <span class="t-mono">[urgent]</span> statique — il n'y a pas d'état submitting/success/error à reproduire
@end

@na
surface: /contact
catégorie: concurrence
raison: /contact ne poste rien — c'est un lien mailto:, pas une form action ni un fetch ; aucune écriture serveur n'est déclenchée, donc pas de double-submit ni de course possible (contrairement à l'hypothèse d'un formulaire de contact)
@end

@case
id: FE-0705
surface: /contact
catégorie: auth-tenant
tag: front
précondition: session anonyme puis connectée
action: charger /contact ; <AppHeader /> (sans prop active) lit page.data.user
attendu: le header reflète l'auth (Sign in/Create account vs Upload/AvatarMenu) ; l'adresse mailto hello@astrophoto.example est identique pour tous — aucune donnée propre au compte (email du user) n'est pré-remplie dans le mailto
@end

@na
surface: /contact
catégorie: intégrité-données
raison: /contact n'écrit aucun message en base (pas de table messages, pas de query!, pas d'action) ; le contact passe par un mailto externe — il n'existe aucune donnée persistée par cette route dont l'intégrité pourrait être compromise
@end

@na
surface: /contact
catégorie: contrat-API
raison: le lien mailto: ne produit aucune requête HTTP vers le backend ; /contact/+page.svelte n'a ni fetch ni action ni +page.server.ts — il n'y a aucun contrat API (statut, schéma de corps, en-têtes) à éprouver pour cette route
@end

@case
id: FE-0706
surface: /contact
catégorie: frontend
tag: front
précondition: page /contact rendue, focus clavier
action: naviguer au clavier (Tab) jusqu'au lien <a href="mailto:hello@astrophoto.example"> coloré var(--accent)
attendu: le lien reçoit un focus visible et s'active à Entrée ; le <span class="t-mono">[urgent]</span> n'est pas focusable (texte non interactif) ; aucun lien mort — mailto valide
@end

@case
id: FE-0707
surface: /contact
catégorie: sécurité
tag: front
précondition: contenu de /contact analysé
action: confirmer que l'adresse hello@astrophoto.example est codée en dur dans le markup et qu'aucun paramètre d'URL n'est interpolé dans le href mailto
attendu: pas d'injection mailto-header possible (sujet/cc non contrôlables par l'URL) ; en-têtes Referrer-Policy: strict-origin-when-cross-origin appliqués par hooks.server.ts ; aucun {@html}
@end

@case
id: FE-0708
surface: /design
catégorie: entrées-limites
tag: front
précondition: page /design rendue (prerender = true), section INPUTS
action: coller une chaîne très longue / caractères Unicode astro (ʰ ᵐ ˢ ° ′) dans l'<Input bind:value={monoInputVal} mono> initialisé à "20ʰ 58ᵐ 47ˢ / +44° 19′"
attendu: le binding $state monoInputVal absorbe la saisie ; le composant Input mono ne tronque ni ne crash ; ces inputs sont une démo locale — la valeur n'est jamais soumise (aucune action liée)
@end

@case
id: FE-0709
surface: /design
catégorie: états
tag: front
précondition: page /design rendue
action: éditer les trois $state de démo — inputVal, monoInputVal, textareaVal (Textarea rows={4}) — puis observer leur réactivité
attendu: chaque frappe met à jour le bind:value correspondant (runes $state) ; il n'y a pas d'état loading/error car la page est prerendue et purement vitrine — l'unique "état" est la valeur locale des champs
@end

@na
surface: /design
catégorie: concurrence
raison: /design est prerender = true (design/+page.ts) et ne déclenche aucune écriture serveur ; les $state inputVal/monoInputVal/textareaVal sont locaux à l'onglet et jamais postés — il n'y a aucune écriture concurrente ni invalidateAll à reproduire
@end

@case
id: FE-0710
surface: /design
catégorie: auth-tenant
tag: front
précondition: session anonyme vs connectée ; noter que /design est prerendu (HTML figé au build)
action: charger /design ; il rend DEUX <AppHeader> (en haut active="Gallery" et un dans la section 8 HEADER & FOOTER) qui lisent page.data.user
attendu: malgré le prérendu, l'auth-state du header s'hydrate côté client depuis page.data.user (commentaire "auth state reflects current session") ; aucune donnée tenant n'apparaît dans le corps vitrine ; la page porte <meta name="robots" content="noindex">
@end

@na
surface: /design
catégorie: intégrité-données
raison: /design n'interagit avec aucune table ni cache ; les données affichées (colorTokens, exifRows NGC 7000, photoSamples) sont des constantes codées en dur dans +page.svelte — aucune donnée persistée, donc aucun invariant d'intégrité à violer
@end

@na
surface: /design
catégorie: contrat-API
raison: /design est prerender = true et n'effectue aucun appel réseau (pas de fetch, pas de load au-delà du export const prerender) ; les composants montrés (Button, Input, ExifTable) reçoivent des props statiques — il n'y a aucun contrat requête/réponse à éprouver
@end

@case
id: FE-0711
surface: /design
catégorie: frontend
tag: front
précondition: viewport étroit (mobile) sur /design
action: rendre la section 9 PHOTO PLACEHOLDERS dont la grille est repeat(auto-fit, minmax(240px, 1fr)) et les swatches COLOR TOKENS flex 0 0 140px
attendu: la grille de <Photo target=...> reflow à une colonne sans débordement ; les <MarkReticle>/<Logo>/<CornerMarks size={14} inset={-8}> rendent leur SVG ; le <main> max-width 1200px reste centré
@end

@case
id: FE-0712
surface: /design
catégorie: sécurité
tag: front
précondition: réponse de /design inspectée + politique CSP Report-Only active
action: /design utilise massivement des attributs style= inline (ex. style="background: {hex}; ...") ; vérifier l'effet sous Content-Security-Policy-Report-Only
attendu: le <meta name="robots" content="noindex"> empêche l'indexation de la vitrine ; les style= inline déclencheraient des rapports CSP style-src si la politique passait en enforce (actuellement style-src inclut 'unsafe-inline', donc Report-Only reste silencieux) — à régulariser avant graduation kit.csp ; aucun {@html} ni script injecté
@end

@na
surface: /privacy
catégorie: entrées-limites
raison: /privacy/+page.svelte n'a aucun champ de saisie ni paramètre lu (pas de form, pas de $page.params, pas de query exploitée) ; il n'existe aucune entrée utilisateur à borner ou fuzzer sur cette page de texte statique
@end

@na
surface: /privacy
catégorie: états
raison: contenu entièrement statique (4 <p> + .t-meta "Last updated: 2026-05-10"), aucun $state ni await dans /privacy/+page.svelte — pas d'état loading/empty/error/success à reproduire
@end

@na
surface: /privacy
catégorie: concurrence
raison: /privacy ne possède ni form action, ni fetch, ni +page.server.ts ; aucune écriture n'est émise depuis cette route, donc aucune course (double-submit, write-write) n'est reproductible
@end

@case
id: FE-0713
surface: /privacy
catégorie: auth-tenant
tag: front
précondition: session anonyme vs connectée
action: charger /privacy ; le corps contient un lien <a href="/settings"> pour export/suppression ; <AppHeader /> lit page.data.user
attendu: le texte et le lien /settings sont rendus pour tous (y compris anonyme) ; suivre /settings en anonyme doit être géré par la garde de /settings, pas par /privacy — /privacy n'effectue aucun contrôle tenant et n'expose aucune donnée de compte
@end

@na
surface: /privacy
catégorie: intégrité-données
raison: /privacy ne lit ni n'écrit de données ; le texte (politique de rétention 30 jours, grâce 30 jours) est codé en dur dans +page.svelte et ne reflète aucune valeur dynamique persistée — il n'y a pas d'invariant de données à éprouver
@end

@na
surface: /privacy
catégorie: contrat-API
raison: /privacy/+page.svelte n'émet aucune requête (pas de fetch, pas d'action, pas de +page.server.ts) ; le seul lien <a href="/settings"> est une navigation interne, pas un appel API — aucun contrat requête/réponse à valider ici
@end

@case
id: FE-0714
surface: /privacy
catégorie: frontend
tag: front
précondition: page /privacy rendue
action: cliquer le lien interne <a href="/settings"> dans le 3e paragraphe ("from settings")
attendu: navigation client SvelteKit vers /settings sans rechargement complet ; le lien n'est pas mort (route /settings existe) ; sur viewport <640px le H1 "What we keep, and why." passe à 32px via la media query
@end

@case
id: FE-0715
surface: /privacy
catégorie: sécurité
tag: front
précondition: réponse SSR de /privacy
action: vérifier les en-têtes appliqués par hooks.server.ts et l'absence de sink XSS
attendu: X-Frame-Options: SAMEORIGIN + Permissions-Policy camera=()/microphone=()/geolocation=() présents ; le lien interne /settings n'a pas de target=_blank donc la question rel=noopener ne s'applique pas ici ; aucun {@html}, aucun contenu utilisateur réfléchi
@end

@na
surface: /terms
catégorie: entrées-limites
raison: /terms/+page.svelte est du texte statique (3 <p> + .t-meta) sans aucun input, paramètre d'URL lu, ni form — il n'y a aucune entrée utilisateur à valider ou pousser aux bornes
@end

@na
surface: /terms
catégorie: états
raison: aucun $state, aucun await, aucun bind dans /terms/+page.svelte ; le contenu (H1 "The short version." + paragraphes) est figé — il n'existe pas d'état loading/empty/error/success à reproduire
@end

@na
surface: /terms
catégorie: concurrence
raison: /terms n'a ni form action, ni fetch, ni +page.server.ts ; aucune écriture serveur n'est déclenchée par cette page, donc aucune situation de concurrence (double-clic, write-write, invalidateAll) n'est possible
@end

@case
id: FE-0716
surface: /terms
catégorie: auth-tenant
tag: front
précondition: session anonyme vs connectée
action: charger /terms ; le 2e paragraphe contient <a href="/settings"> ("from settings") ; <AppHeader /> lit page.data.user
attendu: le texte des conditions et le lien /settings s'affichent identiquement pour anonyme et connecté ; /terms n'applique aucune garde tenant et n'affiche aucune donnée propre au compte — le contrôle d'accès vit dans /settings
@end

@na
surface: /terms
catégorie: intégrité-données
raison: /terms ne lit ni n'écrit aucune donnée ; les affirmations (stockage S3, livraison CloudFront, suppression depuis settings) sont du texte codé en dur dans +page.svelte, non dérivé d'un état persistant — il n'y a aucun invariant de données à compromettre
@end

@na
surface: /terms
catégorie: contrat-API
raison: /terms/+page.svelte ne contient aucun appel réseau (pas de fetch, pas d'action, pas de +page.server.ts) ; le lien <a href="/settings"> est une navigation interne — il n'y a aucun contrat API (statut/schéma de corps) à éprouver sur cette route
@end

@case
id: FE-0717
surface: /terms
catégorie: frontend
tag: front
précondition: viewport <640px puis lien interne
action: rendre /terms en mobile (media query @media max-width: 640px) et activer <a href="/settings"> du 2e paragraphe
attendu: le H1 "The short version." passe de 48px à 32px et le padding à 48px 24px 64px sans débordement ; le lien /settings navigue en interne (route existante, lien non mort)
@end

@case
id: FE-0718
surface: /terms
catégorie: sécurité
tag: front
précondition: réponse SSR de /terms
action: vérifier en-têtes de sécurité + absence de sink
attendu: Content-Security-Policy-Report-Only + X-Content-Type-Options: nosniff + Strict-Transport-Security présents (hooks.server.ts) ; aucun lien externe target=_blank donc rel=noopener non applicable ; aucun {@html}, aucune donnée utilisateur réfléchie dans le markup
@end

<!-- ===== batch G2 ===== -->

@case
id: BE-0800
surface: /api/[...rest]
catégorie: entrées-limites
tag: back
précondition: BODY_SIZE_LIMIT unset/default 512 KB on the frontend Koyeb service; backend route raised its DefaultBodyLimit (e.g. plate-solve re-solve, MAX_XISF_BYTES = 128 MiB)
action: POST a 2 MB body through the proxy so init.body = request.body streams under adapter-node
attendu: adapter-node aborts the incoming stream at BODY_SIZE_LIMIT before reaching axum; SvelteKit returns capital-I `{"message":"Internal Error"}` (no `error` field), distinct from a backend AppError `{"error","message":"internal error"}`
@end

@case
id: BE-0801
surface: /api/[...rest]
catégorie: entrées-limites
tag: back
précondition: proxy live; params.rest comes from the catch-all segment
action: GET /api/explore with a multi-kilobyte query string so `new URL(request.url).search` is appended to targetUrl
attendu: search is forwarded verbatim into `${API}/api/${path}${search}`; oversized querystrings are bounded by the platform header limit, not the proxy, which never length-checks `search`
@end

@case
id: BE-0802
surface: /api/[...rest]
catégorie: états
tag: back
précondition: BACKEND_URL points at a backend that is down/unreachable
action: GET /api/explore through the proxy so `await fetch(targetUrl, init)` rejects
attendu: unlike logout/rss/sitemap there is NO try/catch around the fetch at line 115; the rejection propagates → SvelteKit 500, not a graceful passthrough of an upstream status
@end

@case
id: BE-0803
surface: /api/[...rest]
catégorie: concurrence
tag: back
précondition: two large multi-MB POSTs (photo replace) routed through the proxy in parallel
action: issue both concurrently so each sets init.body = request.body; init.duplex = 'half'
attendu: streaming keeps proxy RSS flat (no per-request arrayBuffer() buffering to ~2× body); concurrency is unbounded at the proxy layer so each is gated only by adapter-node BODY_SIZE_LIMIT, not a shared buffer
@end

@case
id: BE-0804
surface: /api/[...rest]
catégorie: auth-tenant
tag: back
précondition: browser holds the SvelteKit-origin session cookie (`session` in dev / `__Host-session` in prod); request carries no Cookie of its own to the proxy
action: GET /api/photos/mine through the proxy
attendu: inbound `cookie` header is dropped (line 80), then buildCookieHeader(cookies) replays the caller's own jar as `cookie:`; one caller's session is never mixed with another's because cookies come from this request's `cookies` object
@end

@case
id: BE-0805
surface: /api/[...rest]
catégorie: intégrité-données
tag: back
précondition: backend responds with Set-Cookie (signin replay) plus a body stream
action: POST /api/auth/login through the proxy
attendu: Set-Cookie is forwarded verbatim via respHeaders.append (scoped to the SvelteKit origin); body is piped as upstream.body with status/statusText preserved; HOP_BY_HOP (content-length, transfer-encoding) stripped so the re-streamed body is not framed with a stale length
@end

@case
id: BE-0806
surface: /api/[...rest]
catégorie: contrat-API
tag: back
précondition: backend returns a 4xx AppError envelope `{"error":...,"message":...}`
action: GET /api/photos/<missing-id> through the proxy
attendu: proxy passes upstream.status and the JSON envelope through unchanged (return new Response(upstream.body, { status: upstream.status, statusText: upstream.statusText })); it does not rewrite the backend status
@end

@case
id: BE-0807
surface: /api/[...rest]
catégorie: frontend
tag: back
précondition: OPTIONS is exported and mapped to proxy alongside GET/POST/PUT/PATCH/DELETE/HEAD
action: send OPTIONS /api/explore through the proxy
attendu: OPTIONS reaches proxy (not MUTATING, so no origin check); body is not attached (method is not GET/HEAD but undici allows no-body OPTIONS); preflight is forwarded to the backend rather than answered locally — verify CORS contract holds end-to-end
@end

@case
id: BE-0808
surface: /api/[...rest]
catégorie: sécurité
tag: back
précondition: cross-site page drives a bodyless `fetch` carrying the victim's same-origin cookie
action: issue POST /api/photos/<id>/delete with an Origin header of a foreign host
attendu: MUTATING.has(method) true and origin !== selfOrigin → `new Response('cross-origin request blocked', { status: 403 })` before the cookie is replayed; an SSR fetch with no Origin is allowed
@end

@case
id: BE-0809
surface: /api/[...rest]
catégorie: sécurité
tag: back
précondition: params.rest is the unsanitised catch-all segment interpolated into `${API}/api/${path}`
action: GET /api/../admin/secret (or percent-encoded `%2e%2e`) through the proxy to attempt escaping the `/api/` prefix
attendu: path-traversal risk — `path` is concatenated with no normalisation; confirm SvelteKit's router already decodes/normalises `..` so `params.rest` cannot reach a non-/api backend route, else the proxy forwards a traversal upstream
@end

@case
id: BE-0810
surface: /account/logout
catégorie: entrées-limites
tag: back
précondition: caller sends a large/garbage request body to the logout POST
action: POST /account/logout with a multi-MB body
attendu: handler never reads request.body — it only enumerates cookies.getAll() and calls backend /api/auth/logout with method POST and the Cookie header; oversized body is irrelevant to the handler (still capped by adapter-node BODY_SIZE_LIMIT upstream)
@end

@case
id: BE-0811
surface: /account/logout
catégorie: états
tag: back
précondition: user already logged out (no session cookie present)
action: POST /account/logout
attendu: cookies.getAll() yields no session entry; backend /api/auth/logout called with empty/partial Cookie; cookies.delete('session') and cookies.delete('__Host-session') are no-ops; still throws redirect(303, '/') — idempotent
@end

@case
id: BE-0812
surface: /account/logout
catégorie: concurrence
tag: back
précondition: user has two tabs open, both authenticated with the same session
action: POST /account/logout from both tabs near-simultaneously
attendu: both replay the same Cookie to backend /api/auth/logout (idempotent server-side); both clear local cookies and both redirect(303, '/'); no shared frontend state so no race — second call sees the session already invalidated server-side
@end

@case
id: BE-0813
surface: /account/logout
catégorie: auth-tenant
tag: back
précondition: caller A and caller B each have distinct sessions
action: A POSTs /account/logout
attendu: only A's cookies (from A's request `cookies`) are forwarded and deleted; B's session is untouched — there is no global session store in the handler, the cookie jar is per-request
@end

@case
id: BE-0814
surface: /account/logout
catégorie: intégrité-données
tag: back
précondition: prod uses `__Host-session`; dev uses `session`
action: POST /account/logout in either environment
attendu: handler deletes BOTH names (cookies.delete('session') and cookies.delete('__Host-session')) with path:'/' so the cookie is cleared regardless of which prefix the environment stamped; no orphan cookie survives
@end

@case
id: BE-0815
surface: /account/logout
catégorie: contrat-API
tag: back
précondition: only POST is exported from the logout route
action: send GET /account/logout
attendu: SvelteKit returns 405 Method Not Allowed (no GET handler exported); logout is POST-only, preventing drive-by logout via a GET <img>/link (CSRF-by-navigation)
@end

@case
id: BE-0816
surface: /account/logout
catégorie: frontend
tag: back
précondition: logout invoked from a form action / button
action: POST /account/logout and follow the response
attendu: `throw redirect(303, '/')` sends the browser to the home route with a 303 See Other (GET-after-POST); the redirect target is `/`, not the referrer
@end

@case
id: BE-0817
surface: /account/logout
catégorie: sécurité
tag: back
précondition: backend /api/auth/logout is unreachable (network error / 5xx)
action: POST /account/logout while the backend is down
attendu: the try/catch (lines 16-18) swallows the error and falls through to clear cookies locally only; the SERVER-SIDE session is never invalidated, so the leaked/stolen session token remains valid server-side — incomplete logout / session-survival risk
@end

@case
id: BE-0818
surface: /robots.txt
catégorie: entrées-limites
tag: back
précondition: handler is a pure function of `url`; no path params or body
action: GET /robots.txt?with=a-very-long-query-string
attendu: query string is ignored; body is the static DIRECTIVES plus a single `Sitemap: ${origin}/sitemap.xml` line; no input length affects output
@end

@case
id: BE-0819
surface: /robots.txt
catégorie: états
tag: back
précondition: no data dependency — DIRECTIVES is a module constant
action: GET /robots.txt with the backend down / DB empty
attendu: identical text/plain body in every state; robots.txt never calls /api/* so it cannot be affected by feed emptiness or backend availability
@end

@case
id: BE-0820
surface: /robots.txt
catégorie: contrat-API
tag: back
précondition: route serves crawl directives
action: GET /robots.txt
attendu: 200 with `content-type: text/plain; charset=utf-8` and `cache-control: public, max-age=3600`; body ends with `Sitemap: ${origin}/sitemap.xml\n` where origin = `${url.protocol}//${url.host}` (tracks the request host, not a hardcoded staging host)
@end

@case
id: BE-0821
surface: /robots.txt
catégorie: intégrité-données
tag: back
précondition: static file shadows routes under adapter-node
action: verify no frontend/static/robots.txt exists
attendu: the static file must stay deleted or it shadows this route and re-introduces the hardcoded-staging-host bug; integrity of the dynamic Sitemap line depends on the static file's absence
@end

@case
id: BE-0822
surface: /robots.txt
catégorie: sécurité
tag: back
précondition: `origin` derives from attacker-controllable `url.host` (Host header)
action: GET /robots.txt with a spoofed Host header
attendu: the poisoned host lands UNescaped in `Sitemap: ${origin}/sitemap.xml` (text/plain, no escape() applied here) — a Host-injection vector that can advertise an attacker sitemap to crawlers if the platform does not pin Host
@end

@na
surface: /robots.txt
catégorie: concurrence
raison: GET handler is a pure function of `url` with no module-level mutable state (unlike sitemap.xml's `cached` memo and `SITEMAP_TTL_MS`); concurrent requests cannot race.
@end

@na
surface: /robots.txt
catégorie: auth-tenant
raison: robots.txt is a public crawl-directive endpoint; the handler reads no cookies and calls no /api/* route, so there is no session, user, or tenant scoping to test.
@end

@na
surface: /robots.txt
catégorie: frontend
raison: pure text/plain data endpoint returning a `new Response(body, ...)`; no DOM, no Svelte component, no client-side behavior to exercise.
@end

@case
id: BE-0823
surface: /rss.xml
catégorie: entrées-limites
tag: back
précondition: a published photo has a target/title with XML metacharacters and a very long string
action: GET /rss.xml after such a photo is published
attendu: title passes through escape() (& < > " ' → entities) inside `<title>`; very long titles are emitted whole (no truncation), bounded only by the 50-item /api/explore?limit=50 cap
@end

@case
id: BE-0824
surface: /rss.xml
catégorie: états
tag: back
précondition: no published photos, or /api/explore returns non-ok / throws
action: GET /rss.xml with an empty feed
attendu: catch block fails soft → photos = []; lastBuildDate = rfc822(undefined) = current time via `new Date()`; valid RSS with zero `<item>` elements and a populated `<channel>`, never a 500
@end

@case
id: BE-0825
surface: /rss.xml
catégorie: concurrence
tag: back
précondition: many feed readers poll /rss.xml simultaneously
action: issue concurrent GETs
attendu: handler has no module-level mutable state (unlike sitemap's `cached`); each request independently fetches /api/explore?limit=50; the `cache-control: max-age=1800` is a client/edge hint, not a server memo — so concurrency just multiplies /api/explore calls
@end

@case
id: BE-0826
surface: /rss.xml
catégorie: auth-tenant
tag: back
précondition: rss.xml is a public feed
action: GET /rss.xml anonymously
attendu: handler reads no cookies; the internal fetch('/api/explore?limit=50') runs without auth and returns only public/published photos — no per-user scoping; an anonymous reader sees the same feed as everyone
@end

@case
id: BE-0827
surface: /rss.xml
catégorie: intégrité-données
tag: back
précondition: explore items include some with missing short_id or author_handle
action: GET /rss.xml
attendu: `.filter((p) => p.short_id && p.author_handle)` drops incomplete items; guid isPermaLink="true" equals the link `${origin}/u/<handle>/p/<short_id>` so item identity is stable across rebuilds; only published photos (explore source) are listed
@end

@case
id: BE-0828
surface: /rss.xml
catégorie: contrat-API
tag: back
précondition: feed served to RSS clients
action: GET /rss.xml
attendu: 200 with `content-type: application/rss+xml; charset=utf-8` and `cache-control: public, max-age=1800, stale-while-revalidate=3600`; body opens with `<?xml version="1.0" encoding="UTF-8"?>` and rss version="2.0"; pubDate/lastBuildDate are RFC 822 via rfc822() (toUTCString)
@end

@na
surface: /rss.xml
catégorie: frontend
raison: pure application/rss+xml data endpoint built by string concatenation in a RequestHandler; no DOM, no Svelte component, nothing client-side to render or interact with.
@end

@case
id: BE-0830
surface: /rss.xml
catégorie: sécurité
tag: back
précondition: a user sets a handle/display_name/title containing `</title><script>` or `&` or quotes
action: GET /rss.xml
attendu: escape() neutralises all five entities (& < > " ') in title, link, guid, dc:creator, description and enclosure/media url, preventing XML/feed-reader injection; verify NO field is interpolated without escape() — the `pubDate`/`lastBuildDate` are machine-generated so safe, but author-controlled fields must all route through escape()
@end

@case
id: BE-0831
surface: /sitemap.xml
catégorie: entrées-limites
tag: back
précondition: published-photo count exceeds PHOTO_PAGE_CAP*60 (~5,040) / TARGET_PAGE_CAP*60 (~1,200)
action: GET /sitemap.xml on a large dataset
attendu: walkPhotos stops after PHOTO_PAGE_CAP=84 cursor pages, walkTargets after TARGET_PAGE_CAP=20; beyond that, permalinks are silently omitted (the TODO notes a sitemap-index switch is needed) — output stays under Google's 50k/5MB limit by truncation
@end

@case
id: BE-0832
surface: /sitemap.xml
catégorie: états
tag: back
précondition: no published photos and no photographed targets; /api/explore and /api/targets return empty pages
action: GET /sitemap.xml
attendu: walks break on empty page (`if (!cursor || photos.length === 0) break`); urls still contains STATIC_PATHS + CATEGORY_SLUGS (/c/dso etc.); valid `<urlset>` with only the static/category URLs, never a 500
@end

@case
id: BE-0833
surface: /sitemap.xml
catégorie: concurrence
tag: back
précondition: cache cold (`cached` null or expired); two crawler hits arrive together for the same origin
action: issue two concurrent first-GETs to /sitemap.xml
attendu: TOCTOU on the module-level `cached` memo — both miss the `Date.now() < cached.expiresAt` check (lines 89/205), each fans out up to PHOTO_PAGE_CAP+TARGET_PAGE_CAP backend round-trips before either writes `cached`; the memo dedupes only after the first writer lands
@end

@case
id: BE-0834
surface: /sitemap.xml
catégorie: auth-tenant
tag: back
précondition: sitemap is public; explore/targets fetched without auth
action: GET /sitemap.xml anonymously
attendu: only public/published photos and has_photos=true targets are enumerated; no cookies read, no drafts/private resources leaked; the `cached` memo is keyed on `origin` so a dev vs staging host never serves the wrong host's URLs
@end

@case
id: BE-0835
surface: /sitemap.xml
catégorie: intégrité-données
tag: back
précondition: explore page contains items lacking short_id/handle; same handle appears across many photos
action: GET /sitemap.xml
attendu: items without short_id or handle (author_handle ?? owner_handle) are skipped; per-photo profile URL `${origin}/u/<handle>` is pushed repeatedly then collapsed by the `seen` Set dedupe; lastmod = published_at ?? created_at; only published photos listed (no drafts/private)
@end

@case
id: BE-0836
surface: /sitemap.xml
catégorie: contrat-API
tag: back
précondition: sitemap served to crawlers
action: GET /sitemap.xml
attendu: 200 via sitemapResponse with `content-type: application/xml; charset=utf-8` and `cache-control: public, max-age=3600, stale-while-revalidate=86400`; body opens `<?xml ...?>` + `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`; backend limit clamped to 60 so next_cursor is walked
@end

@na
surface: /sitemap.xml
catégorie: frontend
raison: pure application/xml data endpoint assembled by string concatenation; no DOM, no Svelte component, no client-side rendering or interaction to test.
@end

@case
id: BE-0838
surface: /sitemap.xml
catégorie: sécurité
tag: back
précondition: a handle or target slug contains XML metacharacters; Host header is attacker-controllable
action: GET /sitemap.xml with a spoofed Host and a malicious handle/slug present
attendu: every <loc>/<lastmod> value passes through escape() (handles/slugs also encodeURIComponent'd into the path) so no XML injection; but `origin` from `url.host` is still attacker-poisoned and (though escape()-safe) is memoized into `cached` keyed on that origin — Host-injection can poison the cached sitemap URLs until SITEMAP_TTL_MS expires
@end
