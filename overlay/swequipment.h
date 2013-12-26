#ifndef O_SWEQUIPMENT_H
#define O_SWEQUIPMENT_H

typedef struct swequipment swequipment_t;

#include "subwindow.h"
struct swequipment
{
	subwindow_t w;
};

#include "../game.h"

void swequipment_init(swequipment_t* w);
void swequipment_exit(swequipment_t* w);

void swequipment_draw (swequipment_t* w, game_t* g);
char swequipment_catch(swequipment_t* w, game_t* g, float x, float y, int t);

#endif
