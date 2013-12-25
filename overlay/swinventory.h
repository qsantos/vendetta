#ifndef O_SW_INVENTORY_H
#define O_SW_INVENTORY_H

typedef struct swinventory swinventory_t;

#include "subwindow.h"
#include "../game.h"

struct swinventory
{
	subwindow_t w;
};

void swinventory_draw(game_t* g);

#endif
