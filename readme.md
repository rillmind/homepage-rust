<div align="center">

# homepage-rust

Página inicial para o meu servidor caseiro: uma tela só com o estado da máquina,
atalhos e informações do dia a dia, acessível de qualquer dispositivo da rede.

[![Rust](https://img.shields.io/badge/Rust-2024-DEA584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black)](https://react.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-6-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Vite](https://img.shields.io/badge/Vite-8-646CFF?style=flat-square&logo=vite&logoColor=white)](https://vite.dev)
[![Bun](https://img.shields.io/badge/Bun-runtime-000000?style=flat-square&logo=bun&logoColor=white)](https://bun.sh)
[![Catppuccin](https://img.shields.io/badge/tema-Catppuccin-cba6f7?style=flat-square)](https://catppuccin.com)

</div>

## A ideia

Quem tem um servidor em casa acaba com uma dúzia de abas abertas: um painel para ver
uso de memória, outro para ver se o serviço X está de pé, o IP anotado em algum canto.
O objetivo deste projeto é juntar isso numa homepage simples, que eu possa deixar
como página inicial do navegador.

O back-end roda no próprio servidor e lê as informações direto do sistema. O front
consulta essas informações a cada segundo e mostra tudo numa barra no topo e em
blocos na página.

Não é um produto pronto nem pretende competir com Homarr, Dashy e afins. É um projeto
pessoal, feito do jeito que eu quero usar e também para estudar Rust e React. É a continuação do
[homepage](https://github.com/rillmind/homepage), com o back-end reescrito de Go para Rust.

## O que já funciona

- CPU, memória, disco, temperatura, fan e rede em tempo real
- Serviços cadastrados à mão: busca (Ctrl K), filtro por categoria, link e status da porta
- Página Network com todas as interfaces
- Configurações por navegador: título, tema Catppuccin e intervalo de atualização

## Como funciona

```
┌──────────────┐   fetch a cada 1s   ┌───────────────────┐  /proc, /sys  ┌─────────┐
│  front-end   │ ──────────────────► │  back-end (Rust)  │ ────────────► │  Linux  │
│  React/Vite  │ ◄────────────────── │  axum, :2306      │               └─────────┘
└──────────────┘        JSON         └───────────────────┘
```

O front chama `/api` na própria origem, repassado ao back pelo nginx (ou pelo Vite em
desenvolvimento). O back só escuta em `127.0.0.1` e não libera CORS.

**Não há login:** quem abre a página pode editar os serviços. Use só em rede confiável.

### Endpoints

| Método     | Rota               | Para quê                                          |
| ---------- | ------------------ | ------------------------------------------------- |
| GET        | `/stats`           | todos os dados da máquina; `null` é dado ausente  |
| GET, POST  | `/services`        | lista e cria serviços                             |
| PUT, DELETE| `/services/{id}`   | edita e exclui                                    |
| GET        | `/services/status` | status das portas dos serviços, por id            |
| GET        | `/probe/{porta}`   | testa uma porta da máquina                        |

### Configuração

| Variável       | Onde  | Para quê                                                             |
| -------------- | ----- | -------------------------------------------------------------------- |
| `CPU_FAN_PATH` | back  | arquivo `fanN_input` da CPU, se as fans da placa não tiverem label   |
| `DISK_PATH`    | back  | caminho no disco a medir; por padrão, `/`                            |
| `DATA_PATH`    | back  | arquivo dos serviços; por padrão, `data/services.json`               |
| `BIND_ADDR`    | back  | endereço do back; por padrão, `127.0.0.1:2306`                       |

## Rodando

Precisa de Linux, [Rust 1.88+](https://rustup.rs) e [Bun](https://bun.sh).

```sh
git clone https://github.com/rillmind/homepage-rust.git
cd homepage-rust

make install                # dependências do back e do front
make run                    # sobe back e front em segundo plano
```

O back fica em `http://localhost:2306` e o front no endereço que o Vite mostrar
(por padrão `http://localhost:5173`; o log fica em `front-end/dev.log`).

Para parar ou reiniciar:

```sh
make stop
make restart
```

Também dá para controlar cada parte separadamente com `make run-back`,
`make stop-front` etc.

### Com podman

Usa o `podman-compose`. O `podman compose` também serve, mas pode delegar ao docker-compose,
que exige `systemctl --user start podman.socket`.

```sh
podman-compose up --build -d   # front em http://localhost:5173
podman-compose logs -f
podman-compose down            # os serviços ficam em data/services.json
```

Para atualizar, rode `podman-compose down` antes do `up --build -d`: com o container no ar, o
`podman-compose` gera a imagem nova, mas continua usando a antiga.

No docker, crie a pasta antes e informe o seu usuário:

```sh
mkdir -p data && APP_UID=$(id -u) docker compose up --build -d
```

### Serviços

Ficam em `data/services.json`, fora do git. Para reutilizar em outra máquina, copie esse
arquivo e reinicie o back, que só o lê ao iniciar.

## Como testar

```sh
make test   # testes e clippy do back, build e lint do front
```

Para conferir o funcionamento, rode `make run`, consulte um endpoint com
`curl http://localhost:2306/stats` e abra o endereço do Vite.

## Estrutura

```
homepage-rust/
├── Makefile              orquestra back e front
├── docker-compose.yaml   sobe tudo com podman
├── back-end/src/
│   ├── main.rs           servidor HTTP e desligamento
│   ├── pc_info/          dados da máquina
│   └── services/         cadastro e status dos serviços
└── front-end/src/
    ├── lib/              api, polling, formatação e preferências
    └── components/       Nav, Sidebar, Monitor, Services, Network, Settings
```

## Próximos passos
- [ ] Login
- [ ] Página de logs do servidor
