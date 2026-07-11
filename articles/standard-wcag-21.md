---
title: "WCAG 2.1 — Accesibilitate Web (Web Content Accessibility Guidelines)"
description: "Implementarea WCAG 2.1 Level AA în shop-mvp"
date: 2026-07-11
---

# WCAG 2.1 — Accesibilitate Web

## Cuprins

1. [Ce este WCAG](#ce-este-wcag)
2. [Principiile WCAG](#principiile-wcag)
3. [Nivele de conformitate](#nivele-de-conformitate)
4. [Implementare în shop-mvp](#implementare-în-shop-mvp)
5. [Testare și verificare](#testare-și-verificare)

## Ce este WCAG

**WCAG** (Web Content Accessibility Guidelines) este standardul internațional pentru accesibilitatea conținutului web, publicat de W3C. Versiunea curentă este **WCAG 2.2** (2023), iar shop-mvp implementează cerințele **Level AA**.

### De ce e important

- **Legal**: În UE, accesibilitatea web e obligatorie prin *European Accessibility Act* (Directiva 2019/882)
- **Etic**: Toți utilizatorii merită acces egal la conținut
- **Business**: Audiență mai largă (15% din populație are un handicap)
- **SEO**: Multe cerințe WCAG coincid cu bune practici SEO

## Principiile WCAG

### POUR — cele 4 principii fundamentale

| Principiu | Ce înseamnă |
|-----------|-------------|
| **P**ercepibil | Conținutul trebuie să poată fi perceput |
| **O**perabil | Interfața trebuie să poată fi operată |
| **U**șor de înțeles | Conținutul și interfața trebuie să fie inteligibile |
| **R**obust | Conținutul trebuie să funcționeze cu tehnologiile asistive |

## Nivele de conformitate

| Nivel | Puncte de control | Status shop-mvp |
|-------|------------------|-----------------|
| **A** (minim) | 30 | ✅ |
| **AA** (recomandat) | 20 (suplimentar) | ✅ |
| **AAA** (avansat) | 28 (suplimentar) | ⚠️ Parțial |

## Implementare în shop-mvp

### 1. Percepibil

#### 1.1.1 Text alternativ (Level A)

```html
<!-- Toate iconițele au text alternativ sau aria-label -->
<a href="/products" aria-label="Vezi produse">📦</a>
<img src="product.jpg" alt="Telefon Samsung Galaxy S25">
```

#### 1.4.3 Contrast minim (Level AA)

```css
/* Contrast ratio ≥ 4.5:1 pentru text normal */
a {
    color: #3b82f6; /* albastru suficient de închis */
}
.text-gray-600 {
    color: #4b5563; /* contrast 7:1 pe fundal alb */
}
```

#### 1.4.1 Culoare (Level A)

```html
<!-- Nu folosim doar culoarea pentru a transmite informații -->
<span class="text-red-600 font-medium">
    ❌ Stoc epuizat
</span>
```

### 2. Operabil

#### 2.1.1 Keyboard (Level A)

```html
<!-- Toate elementele interactive sunt focusabile cu Tab -->
<a href="/products" class="focus:outline-2 focus:outline-blue-500 rounded">
    Produse
</a>

<!-- Skip-to-content link pentru navigare rapidă -->
<a href="#main-content" class="sr-only focus:not-sr-only ...">
    Sari la conținut
</a>
```

#### 2.4.1 Skip-to-content (Level A)

```html
<!-- Primul element din <body> e un link "sari la conținut" -->
<a href="#main-content" class="sr-only focus:not-sr-only ...">
    Sari la conținut
</a>
<main id="main-content" role="main">
    <!-- conținutul principal -->
</main>
```

#### 2.4.6 Headings and Labels (Level AA)

```html
<!-- Formulare cu label-uri explicite -->
<label for="search-input" class="sr-only">Caută produse</label>
<input id="search-input" type="text" name="q" placeholder="Caută produse...">
```

### 3. Ușor de înțeles

#### 3.1.1 Limba paginii (Level A)

```html
<html lang="ro">
```

#### 3.3.2 Etichete și instrucțiuni (Level A)

```html
<form action="/search" method="GET" role="search" aria-label="Caută produse">
    <label for="search-input">Caută produse</label>
    <input id="search-input" type="text" name="q" required>
</form>
```

### 4. Robuste

#### 4.1.2 Name, Role, Value (Level A)

```html
<nav role="navigation" aria-label="Navigare principală">
    <!-- link-uri de navigare -->
</nav>
<main id="main-content" role="main">
    <!-- conținut principal -->
</main>
<footer role="contentinfo">
    <!-- informații footer -->
</footer>
```

#### 4.1.3 Status Messages (Level AA)

```html
<!-- Mesajele de eroare sunt anunțate de screen reader -->
<div role="alert" class="text-red-600">
    ❌ Produsul nu este disponibil
</div>
```

## Testare și verificare

### Instrumente

| Instrument | Scop |
|-----------|------|
| **axe DevTools** | Audit automat accesibilitate |
| **WAVE** | Evaluare vizuală erori |
| **Lighthouse** | Scor accesibilitate în Chrome |
| **NVDA / VoiceOver** | Screen reader testing |
| **Tab navigation** | Testare tastatură manuală |

### Comenzi rapide pentru testare

```bash
# Verifică headerele de accesibilitate
curl -s http://localhost:3001/ | grep -i "role=\|aria-\|lang="

# Verifică skip-to-content
curl -s http://localhost:3001/ | grep -i "skip\|#main-content"
```

### Checklist Level AA

- [x] Lang attribute pe `<html>`
- [x] Skip-to-content link
- [x] ARIA roles: banner, navigation, main, contentinfo
- [x] ARIA labels pe navigare și formulare
- [x] Focus visible pe toate elementele interactive
- [x] Form labels explicite
- [x] Contrast ratio ≥ 4.5:1
- [x] Nu doar culoare pentru informații
- [x] Keyboard navigable (Tab, Enter, Escape)
- [x] respects `prefers-reduced-motion`

## Referințe

- [WCAG 2.1 Overview (W3C)](https://www.w3.org/WAI/standards-guidelines/wcag/)
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [European Accessibility Act](https://ec.europa.eu/social/main.jsp?catId=1202)
