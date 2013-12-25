#ifndef O_SWSKILLS_H
#define O_SWSKILLS_H

typedef struct swskills swskills_t;

#include "subwindow.h"
#include "../game.h"

struct swskills
{
	subwindow_t w;
};

void swskills_draw (game_t* g);
char swskills_catch(game_t* g, float x, float y);

#endif
