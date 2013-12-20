#ifndef DRAW_H
#define DRAW_H

#include <SFML/Graphics.h>

#include "world.h"

void draw_character(sfRenderWindow* render, character_t* c);
void draw_mine     (sfRenderWindow* render, mine_t* m);
void draw_world    (sfRenderWindow* render, world_t* w);

#endif
