#include <SDL2/SDL.h>
#include <SDL2/SDL_ttf.h>

/* kov-lang blog palette — monochrome, depth through grey shades only
   L0 #070707  deepest bg
   L1 #0c0c0c  raised surface
   L2 #131313  card/code bg
   L3 #1a1a1a  border
   L4 #252525  strong border
   L5 #333333  dim
   L6 #4a4a4a  muted
   L7 #777777  body
   L8 #aaaaaa  titles
   L9 #cccccc  emphasis
   LA #e8e8e8  bright */

#define W  880
#define H  600
#define L0 0x07,0x07,0x07
#define L1 0x0c,0x0c,0x0c
#define L2 0x13,0x13,0x13
#define L3 0x1a,0x1a,0x1a
#define L4 0x25,0x25,0x25
#define L5 0x33,0x33,0x33
#define L6 0x4a,0x4a,0x4a
#define L7 0x77,0x77,0x77
#define L8 0xaa,0xaa,0xaa
#define L9 0xcc,0xcc,0xcc
#define LA 0xe8,0xe8,0xe8

static TTF_Font *f12, *f13, *f15, *f20, *f36;

static void tx(SDL_Renderer *r, TTF_Font *f, int x, int y,
               Uint8 R, Uint8 G, Uint8 B, const char *s) {
    SDL_Color c = {R, G, B, 255};
    SDL_Surface *sf = TTF_RenderUTF8_Blended(f, s, c);
    if (!sf) return;
    SDL_Texture *t = SDL_CreateTextureFromSurface(r, sf);
    SDL_Rect d = {x, y, sf->w, sf->h};
    SDL_RenderCopy(r, t, NULL, &d);
    SDL_DestroyTexture(t);
    SDL_FreeSurface(sf);
}

static void dashed_h(SDL_Renderer *r, int x, int y, int w) {
    SDL_SetRenderDrawColor(r, L3, 255);
    for (int i = x; i < x + w; i += 6)
        SDL_RenderDrawLine(r, i, y, i + 3, y);
}

static void rect_fill(SDL_Renderer *r, int x, int y, int w, int h,
                       Uint8 R, Uint8 G, Uint8 B) {
    SDL_SetRenderDrawColor(r, R, G, B, 255);
    SDL_Rect rc = {x, y, w, h};
    SDL_RenderFillRect(r, &rc);
}

/* draw dot grid like the blog's body::before */
static void dot_grid(SDL_Renderer *r) {
    SDL_SetRenderDrawColor(r, L4, 40);
    for (int y = 0; y < H; y += 24)
        for (int x = 0; x < W; x += 24)
            SDL_RenderDrawPoint(r, x, y);
}

int main(void) {
    SDL_Init(SDL_INIT_VIDEO);
    TTF_Init();

    const char *fp = "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf";
    const char *fb = "/usr/share/fonts/TTF/JetBrainsMonoNerdFontMono-Bold.ttf";
    f12 = TTF_OpenFont(fp, 11);
    f13 = TTF_OpenFont(fp, 13);
    f15 = TTF_OpenFont(fp, 15);
    f20 = TTF_OpenFont(fb, 20);
    f36 = TTF_OpenFont(fb, 36);

    SDL_Window *win = SDL_CreateWindow("c2v",
        SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
        W, H, SDL_WINDOW_SHOWN | SDL_WINDOW_ALLOW_HIGHDPI);
    SDL_Renderer *r = SDL_CreateRenderer(win, -1,
        SDL_RENDERER_ACCELERATED | SDL_RENDERER_PRESENTVSYNC);

    int run = 1;
    int mx = 32, y;

    while (run) {
        SDL_Event e;
        while (SDL_PollEvent(&e)) {
            if (e.type == SDL_QUIT) run = 0;
            if (e.type == SDL_KEYDOWN && e.key.keysym.sym == SDLK_ESCAPE) run = 0;
        }

        SDL_SetRenderDrawColor(r, L0, 255);
        SDL_RenderClear(r);
        dot_grid(r);

        y = 28;

        /* source — one line, dim */
        tx(r, f12, mx, y, L6, "fd_sum16.c");
        tx(r, f13, mx, y + 20, L8, "for( i = 0; i < 16; i++ )  sum += v[i];");

        y += 56;
        dashed_h(r, mx, y, W - mx * 2);

        /* redundancy — the measurement */
        y += 16;
        tx(r, f12, mx, y, L6, "redundant ordering");
        y += 18;
        tx(r, f36, mx, y, LA, "82.9");
        tx(r, f15, mx + 150, y + 16, L6, "bits");
        tx(r, f13, mx + 210, y + 18, L5, "42% of ordering is free");

        y += 56;
        dashed_h(r, mx, y, W - mx * 2);

        /* chain detail — sparse */
        y += 16;
        tx(r, f12, mx, y, L6, "chain");
        y += 18;
        tx(r, f15, mx, y, L8, "Add");
        tx(r, f13, mx + 60, y, L6, "17 ops");
        tx(r, f13, mx + 160, y, L7, "depth 16 → 5");
        tx(r, f13, mx + 380, y, L6, "11 levels recoverable");

        y += 36;
        dashed_h(r, mx, y, W - mx * 2);

        /* before / after bars */
        y += 18;
        tx(r, f12, mx, y, L6, "before");
        int bar_x = mx + 80;
        int bar_w_full = W - bar_x - mx - 80;
        rect_fill(r, bar_x, y, bar_w_full, 14, L4);
        tx(r, f12, bar_x + bar_w_full + 10, y + 1, L7, "22 cyc");

        y += 28;
        tx(r, f12, mx, y, L6, "after");
        int bar_w_half = bar_w_full / 2;
        rect_fill(r, bar_x, y, bar_w_half, 14, L8);
        tx(r, f12, bar_x + bar_w_half + 10, y + 1, L9, "11 cyc");

        y += 36;
        dashed_h(r, mx, y, W - mx * 2);

        /* predicted vs actual */
        y += 20;
        tx(r, f12, mx, y, L6, "predicted");
        tx(r, f12, mx + 300, y, L6, "actual");

        y += 18;
        tx(r, f36, mx, y, L9, "2.10x");
        tx(r, f36, mx + 300, y, LA, "2.10x");

        y += 56;
        dashed_h(r, mx, y, W - mx * 2);

        /* verified */
        y += 16;
        tx(r, f13, mx, y, L7, "1000/1000 correct");
        tx(r, f12, mx + 200, y + 2, L5, "C reference vs generated verilog");

        /* bottom tag */
        tx(r, f12, mx, H - 24, L5, "c2v 0.1.0");

        SDL_RenderPresent(r);
    }

    TTF_CloseFont(f12);
    TTF_CloseFont(f13);
    TTF_CloseFont(f15);
    TTF_CloseFont(f20);
    TTF_CloseFont(f36);
    TTF_Quit();
    SDL_DestroyRenderer(r);
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}
