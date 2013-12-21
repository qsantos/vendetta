#ifndef DRAW_H
#define DRAW_H

#include "world/world.h"
#include "graphics.h"

void draw_character(graphics_t* g, character_t* c);
void draw_mine     (graphics_t* g, mine_t* m);
void draw_world    (graphics_t* g, world_t* w);

#endif
