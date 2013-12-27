#ifndef O_SWSKILLS_H
#define O_SWSKILLS_H

typedef struct swskills swskills_t;

#include "subwindow.h"

struct swskills
{
	subwindow_t w;
};

#include "../game.h"

void swskills_init(swskills_t* w, graphics_t* g);
void swskills_exit(swskills_t* w);

char swskills_cursor(swskills_t* w, game_t* g, int x, int y);
void swskills_draw  (swskills_t* w, game_t* g);
char swskills_catch (swskills_t* w, game_t* g, int x, int y, int t);

#endif
