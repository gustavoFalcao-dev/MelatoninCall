// ========================================
// CALL - LOGIN
// ========================================

import { apiPost } from "./utils/api.js";

document.getElementById('login-form').addEventListener('submit', async (e) => {
    e.preventDefault();

    const username = document.getElementById('username').value.trim();
    const password = document.getElementById('password').value;
    const message = document.getElementById('error-message');

    if (!username || !password) {
        message.style.color = "#ffb0a5";
        message.textContent = "PREENCHA USUÁRIO E SENHA.";
        return;
    }

    try {
        const data = await apiPost('/users/login', { username, password });
        window.location.href = 'app.html';
    } catch (err) {
        message.style.color = "#ffb0a5";
        message.textContent = err.message;
    }
});

// ========================================
// EVENTOS
// ========================================

document.getElementById("go-register").addEventListener("click", () => {
    window.location.href = "register.html";
});