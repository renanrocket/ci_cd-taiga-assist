// Navegação entre abas e seções de Configurações a partir de qualquer tela (botões "Configurar o Kiro", etc.).
export type Aba = "pasta" | "cards" | "config";
export type SecaoConfig = "geral" | "taiga" | "git" | "pastas" | "kiro" | "requisitos";

class Nav {
  aba = $state<Aba>("pasta");
  secaoPedida = $state<SecaoConfig | null>(null);

  ir(aba: Aba, secao: SecaoConfig | null = null) {
    this.secaoPedida = secao;
    this.aba = aba;
  }
}

export const nav = new Nav();
