#ifndef O_SWBUILDING_H
#define O_SWBUILDING_H

typedef struct swbuilding swbuilding_t;

#include "subwindow.h"
#include "../game.h"

struct swbuilding
{
	subwindow_t w;
};

void swbuilding_draw (game_t* g);
char swbuilding_catch(game_t* g, float x, float y);

#endif
