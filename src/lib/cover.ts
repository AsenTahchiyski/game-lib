// onerror handler for cover <img>s. Steam's portrait art (library_600x900) is
// missing for some apps, so fall back to the header, then the capsule, then hide.
export function coverFallback(e: Event) {
  const img = e.currentTarget as HTMLImageElement;
  const src = img.src;
  if (src.includes("/library_600x900.jpg")) {
    img.src = src.replace("/library_600x900.jpg", "/header.jpg");
  } else if (src.includes("/header.jpg")) {
    img.src = src.replace("/header.jpg", "/capsule_616x353.jpg");
  } else {
    img.style.display = "none";
  }
}
