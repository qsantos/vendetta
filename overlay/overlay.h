#ifndef OVERLAY_H
#define OVERLAY_H

typedef struct overlay overlay_t;

#include "../game.h"

struct overlay
{
	kindOf_building_t* selectedBuilding;
};

overlay_t* overlay_init(void);
void       overlay_exit(overlay_t* o);

int overlay_catch(game_t* g, float x, float y);

void draw_overlay(game_t* g);

#endif
