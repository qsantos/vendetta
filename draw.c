#include "draw.h"

#include <stdlib.h>

void draw_character(sfRenderWindow* render, character_t* c)
{
	static sfTexture* texture = NULL;
	static sfSprite*  sprite  = NULL;
	
	if (texture == NULL)
	{
		texture = sfTexture_createFromFile("character.png", NULL);
		if (texture == NULL)
			exit(1);

		sprite = sfSprite_create();
		sfSprite_setTexture(sprite, texture, sfTrue);
		sfIntRect rect = {24*1, 32*2, 24, 32};
		sfSprite_setTextureRect(sprite, rect);

	}

	sfVector2f pos = { c->x, c->y };
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(render, sprite, NULL);
}

void draw_world(sfRenderWindow* render, world_t* w)
{
	draw_character(render, &w->characters[0]);
}
