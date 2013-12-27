#ifndef O_SWEQUIPMENT_H
#define O_SWEQUIPMENT_H

typedef struct swequipment swequipment_t;

#include "subwindow.h"

struct swequipment
{
	subwindow_t w;
};

#include "../game.h"

void swequipment_init(swequipment_t* w, graphics_t* g);
void swequipment_exit(swequipment_t* w);

char swequipment_cursor(swequipment_t* w, game_t* g, int x, int y);
void swequipment_draw  (swequipment_t* w, game_t* g);
char swequipment_catch (swequipment_t* w, game_t* g, int x, int y, int t);

#endif
