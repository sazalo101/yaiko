const state = {
  profile: null,
};

const profileForm = document.querySelector("#profile-form");
const linkForm = document.querySelector("#link-form");
const linkList = document.querySelector("#link-list");
const template = document.querySelector("#editor-link-template");
const toast = document.querySelector("#toast");

async function request(url, options = {}) {
  const response = await fetch(url, {
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
    ...options,
  });

  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || `Request failed with status ${response.status}`);
  }
  return data;
}

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("is-visible");
  clearTimeout(showToast.timeoutId);
  showToast.timeoutId = setTimeout(() => toast.classList.remove("is-visible"), 2200);
}

function paintPreview(profile) {
  document.documentElement.style.setProperty("--accent", profile.accent_color);
  document.querySelector("#preview-avatar").textContent = profile.avatar_emoji || "✨";
  document.querySelector("#preview-slug").textContent = `@${profile.slug}`;
  document.querySelector("#preview-name").textContent = profile.display_name;
  document.querySelector("#preview-headline").textContent = profile.headline;
  document.querySelector("#preview-bio").textContent = profile.bio;
  document.querySelector("#preview-updated").textContent = `Updated ${new Date(profile.updated_at).toLocaleString()}`;

  const linksNode = document.querySelector("#preview-links");
  linksNode.innerHTML = "";
  profile.links.forEach((link) => {
    const anchor = document.createElement("a");
    anchor.className = `preview-link${link.featured ? " featured" : ""}`;
    anchor.href = link.url;
    anchor.target = "_blank";
    anchor.rel = "noreferrer";
    anchor.innerHTML = `<strong>${escapeHtml(link.title)}</strong><span>${escapeHtml(link.description || link.url)}</span>`;
    linksNode.appendChild(anchor);
  });
}

function fillProfileForm(profile) {
  profileForm.display_name.value = profile.display_name;
  profileForm.headline.value = profile.headline;
  profileForm.bio.value = profile.bio;
  profileForm.avatar_emoji.value = profile.avatar_emoji;
  profileForm.accent_color.value = profile.accent_color;
}

function renderLinkEditor(profile) {
  linkList.innerHTML = "";

  if (!profile.links.length) {
    const empty = document.createElement("p");
    empty.className = "lede";
    empty.textContent = "No links yet. Add your first destination above.";
    linkList.appendChild(empty);
    return;
  }

  profile.links.forEach((link) => {
    const node = template.content.firstElementChild.cloneNode(true);
    node.id.value = link.id;
    node.title.value = link.title;
    node.url.value = link.url;
    node.description.value = link.description;
    node.featured.checked = !!link.featured;

    node.addEventListener("submit", async (event) => {
      event.preventDefault();
      const payload = {
        title: node.title.value,
        url: node.url.value,
        description: node.description.value,
        featured: node.featured.checked,
      };

      try {
        const data = await request(`/api/links/${link.id}`, {
          method: "PUT",
          body: JSON.stringify(payload),
        });
        updateState(data.profile, "Link updated");
      } catch (error) {
        showToast(error.message);
      }
    });

    node.querySelector(".delete-link").addEventListener("click", async () => {
      try {
        const data = await request(`/api/links/${link.id}`, {
          method: "DELETE",
        });
        updateState(data.profile, "Link deleted");
      } catch (error) {
        showToast(error.message);
      }
    });

    linkList.appendChild(node);
  });
}

function updateState(profile, toastMessage) {
  state.profile = profile;
  fillProfileForm(profile);
  paintPreview(profile);
  renderLinkEditor(profile);
  if (toastMessage) {
    showToast(toastMessage);
  }
}

profileForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const payload = {
    display_name: profileForm.display_name.value,
    headline: profileForm.headline.value,
    bio: profileForm.bio.value,
    avatar_emoji: profileForm.avatar_emoji.value,
    accent_color: profileForm.accent_color.value,
  };

  try {
    const data = await request("/api/profile", {
      method: "PUT",
      body: JSON.stringify(payload),
    });
    updateState(data.profile, "Profile updated");
  } catch (error) {
    showToast(error.message);
  }
});

linkForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const payload = {
    title: linkForm.title.value,
    url: linkForm.url.value,
    description: linkForm.description.value,
    featured: linkForm.featured.checked,
  };

  try {
    const data = await request("/api/links", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    updateState(data.profile, "Link added");
    linkForm.reset();
  } catch (error) {
    showToast(error.message);
  }
});

async function bootstrap() {
  try {
    const data = await request("/api/profile");
    updateState(data.profile);
  } catch (error) {
    showToast(error.message);
  }
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

bootstrap();
