#ifndef O_SW_INVENTORY_H
#define O_SW_INVENTORY_H

typedef struct swinventory swinventory_t;

#include "subwindow.h"

struct swinventory
{
	subwindow_t w;
};

#include "../game.h"

void swinventory_init(swinventory_t* w, graphics_t* g);
void swinventory_exit(swinventory_t* w);

char swinventory_cursor(swinventory_t* w, game_t* g, int x, int y);
void swinventory_draw  (swinventory_t* w, game_t* g);
char swinventory_catch (swinventory_t* w, game_t* g, int x, int y, int t);

#endif
