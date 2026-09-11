// ========================================
// CALL - CADASTRO
// ========================================

import { apiPost } from "./utils/api.js";

// Troca entre as telas de cadastro e sucesso.
function showScreen(screenId) {
    document.querySelectorAll(".screen").forEach(screen => {
        screen.classList.remove("active");
    });

    document.getElementById(screenId).classList.add("active");

    document.getElementById("register-message").textContent = "";
}

document.getElementById("register-form").addEventListener("submit", async (e) => {
    e.preventDefault();

    const username = document.getElementById("username").value.trim();
    const email = document.getElementById("email").value.trim();
    const password = document.getElementById("password").value;
    const confirmPassword = document.getElementById("password-confirm").value;
    const message = document.getElementById("register-message");

    if (!username || !email || !password || !confirmPassword) {
        message.textContent = "PREENCHA TODOS OS CAMPOS.";
        return;
    }

    if (username.length < 3) {
        message.textContent = "O USUÁRIO DEVE TER PELO MENOS 3 CARACTERES.";
        return;
    }

    if (password.length < 8) {
        message.textContent = "A SENHA DEVE TER PELO MENOS 8 CARACTERES.";
        return;
    }

    if (password !== confirmPassword) {
        message.textContent = "AS SENHAS NÃO SÃO IGUAIS.";
        return;
    }

    try {
        await apiPost('/users/register', { username, email, password });

        // Limpa os campos após o cadastro.
        document.getElementById("username").value = "";
        document.getElementById("email").value = "";
        document.getElementById("password").value = "";
        document.getElementById("password-confirm").value = "";

        // Vai para a tela de sucesso.
        showScreen("success-screen");
    } catch (err) {
        message.textContent = err.message.toUpperCase();
    }
});

// ========================================
// EVENTOS
// ========================================

document.getElementById("back-login").addEventListener("click", () => {
    window.location.href = "login.html";
});

document.getElementById("success-login").addEventListener("click", () => {
    window.location.href = "index.html";
});