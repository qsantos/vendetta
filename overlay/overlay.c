#include "overlay.h"

#include <stdlib.h>
#include <stdio.h>

#define PANEL_N_COLS 3

void draw_overlay(game_t* g)
{
	static sfTexture* texture = NULL;
	static sfSprite*  sprite  = NULL;
	
	if (texture == NULL)
	{
		texture = sfTexture_createFromFile("buildings.png", NULL);
		if (texture == NULL)
			exit(1);

		sprite = sfSprite_create();
		sfSprite_setTexture(sprite, texture, sfTrue);
	}

	sfVector2f pos = {0, 0};
	for (int i = 0; i < g->u.n_buildings; i++)
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

		sfRenderWindow_drawSprite(g->g.render, sprite, NULL);
	}
}

int overlay_catch(game_t* g, float x, float y)
{
	(void) g;

	int i = y / 28;
	int j = x / 28;
	int id = PANEL_N_COLS*i + j;
	if (j < PANEL_N_COLS && id < g->u.n_buildings)
	{
		printf("%i\n", id);
		return 1;
	}

	return 0;
}
