#include "overlay.h"

#include <stdlib.h>
#include <stdio.h>

#include "../util.h"

#define PANEL_N_COLS 3

overlay_t* overlay_init(void)
{
	overlay_t* o = CALLOC(overlay_t, 1);
	o->selectedBuilding = NULL;
	return o;
}

void overlay_exit(overlay_t* o)
{
	free(o);
}

void draw_overlay(game_t* g)
{
	static sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(g->g, "buildings.png");
		sprite = g->g->sprites[id];
	}

	sfVector2f pos = {0, 0};
	for (int i = 0; i < g->u->n_buildings; i++)
	{
		sfIntRect rect = {28*i, 28*1, 28, 28};
		sfSprite_setTextureRect(sprite, rect);

		sfSprite_setPosition(sprite, pos);

		pos.x += 28;
		if (pos.x >= 28*PANEL_N_COLS)
		{
			pos.x = 0;
			pos.y += 28;
		}

		sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
	}
}

int overlay_catch(game_t* g, float x, float y)
{
	int i = y / 28;
	int j = x / 28;
	int id = PANEL_N_COLS*i + j;
	if (j < PANEL_N_COLS && id < g->u->n_buildings)
	{
		g->o->selectedBuilding = &g->u->buildings[id];
		return 1;
	}

	return 0;
}
