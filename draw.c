#include "draw.h"

#include <stdlib.h>

void draw_object(sfRenderWindow* render, object_t* o, sfSprite* sprite)
{
	sfVector2f pos = {o->x - o->w / 2, o->y - o->h};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(render, sprite, NULL);
}

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

	draw_object(render, &c->o, sprite);
}

void draw_mine(sfRenderWindow* render, mine_t* m)
{
	static sfTexture* texture = NULL;
	static sfSprite*  sprite  = NULL;
	
	if (texture == NULL)
	{
		texture = sfTexture_createFromFile("mines.png", NULL);
		if (texture == NULL)
			exit(1);

		sprite = sfSprite_create();
		sfSprite_setTexture(sprite, texture, sfTrue);
		sfIntRect rect = {32*3, 32*0, 32, 32};
		sfSprite_setTextureRect(sprite, rect);
	}

	draw_object(render, &m->o, sprite);
}

void draw_world(sfRenderWindow* render, world_t* w)
{
	for (size_t i = 0; i < 10; i++)
		draw_mine(render, &w->mines[i]);
	draw_character(render, &w->characters[0]);
}
