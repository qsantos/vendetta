#include "draw.h"

#include <stdlib.h>

void draw_object(graphics_t* g, object_t* o, sfSprite* sprite)
{
	sfVector2f pos = {o->x - o->w/2, o->y - o->h};
	sfSprite_setPosition(sprite, pos);
	sfRenderWindow_drawSprite(g->render, sprite, NULL);
}

void draw_character(graphics_t* g, character_t* c)
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

	}

	sfIntRect rect = {24*1, 32*c->dir, 24, 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &c->o, sprite);
}

void draw_mine(graphics_t* g, mine_t* m)
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
	}

	int t = m->t->id;
	sfIntRect rect = {32*t, 32*0, 32, 32};
	sfSprite_setTextureRect(sprite, rect);
	draw_object(g, &m->o, sprite);
}

void draw_world(graphics_t* g, world_t* w)
{
	for (size_t i = 0; i < 10; i++)
		draw_mine(g, &w->mines[i]);
	draw_character(g, &w->characters[0]);
}
